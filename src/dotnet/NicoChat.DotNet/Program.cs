using System.Net;
using System.Net.Http.Json;
using Microsoft.Extensions.FileProviders;

var builder = WebApplication.CreateBuilder(args);
var webRootPath = Path.GetFullPath(Path.Combine(builder.Environment.ContentRootPath, "..", "..", "..", "web"));

builder.WebHost.UseUrls(Environment.GetEnvironmentVariable("ASPNETCORE_URLS") ?? "http://0.0.0.0:5000");
builder.Services.AddHttpClient<ChatGateway>();

var app = builder.Build();
var webRoot = new PhysicalFileProvider(webRootPath);

app.UseDefaultFiles(new DefaultFilesOptions
{
    FileProvider = webRoot
});
app.UseStaticFiles(new StaticFileOptions
{
    FileProvider = webRoot
});

app.MapGet("/api/health", (ChatGateway gateway) =>
    Results.Ok(new HealthResponse("ok", gateway.Backend, gateway.Model, gateway.Mode)));

app.MapGet("/api/models", async (ChatGateway gateway, CancellationToken cancellationToken) =>
{
    var models = await gateway.GetModelsAsync(cancellationToken);
    return Results.Ok(new ModelsResponse(models));
});

app.MapPost("/api/chat", async Task<IResult> (ChatRequest request, ChatGateway gateway, CancellationToken cancellationToken) =>
{
    if (request.Messages is not { Count: > 0 })
    {
        return Results.BadRequest(new ErrorResponse("Please send at least one message."));
    }

    if (!request.Messages.Any(message =>
            string.Equals(message.Role, "user", StringComparison.OrdinalIgnoreCase) &&
            !string.IsNullOrWhiteSpace(message.Content)))
    {
        return Results.BadRequest(new ErrorResponse("At least one non-empty user message is required."));
    }

    var model = string.IsNullOrWhiteSpace(request.Model) ? gateway.Model : request.Model;

    try
    {
        var content = await gateway.GetReplyAsync(request.Messages, model, cancellationToken);
        return Results.Ok(new ChatReply("assistant", content, gateway.Mode, model));
    }
    catch (HttpRequestException exception)
    {
        return Results.Problem(
            title: "Local AI backend unavailable",
            detail: gateway.ToClientErrorMessage(exception),
            statusCode: StatusCodes.Status503ServiceUnavailable);
    }
});

app.Run();

sealed class ChatGateway(HttpClient httpClient)
{
    private readonly HttpClient _httpClient = httpClient;
    private readonly string _ollamaUrl = (Environment.GetEnvironmentVariable("OLLAMA_URL") ?? "http://127.0.0.1:11434").TrimEnd('/');
    private readonly bool _useMock = bool.TryParse(Environment.GetEnvironmentVariable("NICOCHAT_USE_MOCK"), out var useMock) && useMock;

    public string Backend => ".NET";
    public string Model { get; } = Environment.GetEnvironmentVariable("OLLAMA_MODEL") ?? "qwen3";
    public string Mode => _useMock ? "mock" : "ollama";

    public string ToClientErrorMessage(HttpRequestException? exception = null)
    {
        if (_useMock)
        {
            return "Mock mode is enabled.";
        }

        if (!string.IsNullOrWhiteSpace(exception?.Message))
        {
            return exception!.Message;
        }

        return $"Unable to reach local Ollama instance at {_ollamaUrl}. " +
               "Start it with 'ollama serve', verify connectivity with 'ollama list', or set NICOCHAT_USE_MOCK=true.";
    }

    public async Task<IReadOnlyList<string>> GetModelsAsync(CancellationToken cancellationToken)
    {
        if (_useMock)
        {
            return [Model];
        }

        try
        {
            var tags = await _httpClient.GetFromJsonAsync<OllamaTagsResponse>(
                $"{_ollamaUrl}/api/tags", cancellationToken);
            var names = tags?.Models?.Select(m => m.Name).Where(n => !string.IsNullOrWhiteSpace(n)).ToList();
            return names is { Count: > 0 } ? names! : [];
        }
        catch
        {
            return [];
        }
    }

    public async Task<string> GetReplyAsync(IReadOnlyList<ChatMessage> messages, string model, CancellationToken cancellationToken)
    {
        if (_useMock)
        {
            return BuildMockReply(messages);
        }

        _httpClient.Timeout = TimeSpan.FromSeconds(90);

        using var response = await _httpClient.PostAsJsonAsync(
            $"{_ollamaUrl}/api/chat",
            new OllamaRequest(model, false, messages.Select(message =>
                new ChatMessage(message.Role.ToLowerInvariant(), message.Content)).ToArray()),
            cancellationToken);

        if (!response.IsSuccessStatusCode)
        {
            var body = await response.Content.ReadAsStringAsync(cancellationToken);

            if (response.StatusCode == HttpStatusCode.NotFound &&
                body.Contains("not found", StringComparison.OrdinalIgnoreCase))
            {
                throw new HttpRequestException(
                    $"Selected model '{model}' is not installed in Ollama. Run 'ollama list' and choose an available model from the GUI.",
                    null,
                    response.StatusCode);
            }

            throw new HttpRequestException(
                $"Ollama responded with {(int)response.StatusCode} {response.StatusCode}: {body}",
                null,
                response.StatusCode);
        }

        var payload = await response.Content.ReadFromJsonAsync<OllamaResponse>(cancellationToken: cancellationToken);
        var content = payload?.Message?.Content?.Trim();

        if (string.IsNullOrWhiteSpace(content))
        {
            throw new HttpRequestException("Ollama returned an empty response.", null, HttpStatusCode.BadGateway);
        }

        return content;
    }

    private static string BuildMockReply(IReadOnlyList<ChatMessage> messages)
    {
        var lastUserMessage = messages.Last(message =>
            string.Equals(message.Role, "user", StringComparison.OrdinalIgnoreCase));

        return $"[Mock .NET] You said: \"{lastUserMessage.Content.Trim()}\". " +
               $"Conversation length: {messages.Count} message(s).";
    }
}

sealed record ChatRequest(IReadOnlyList<ChatMessage> Messages, string? Model = null);

sealed record ChatMessage(string Role, string Content);

sealed record ChatReply(string Role, string Content, string Mode, string Model);

sealed record ErrorResponse(string Error);

sealed record HealthResponse(string Status, string Backend, string Model, string Mode);

sealed record ModelsResponse(IReadOnlyList<string> Models);

sealed record OllamaTagsResponse(IReadOnlyList<OllamaModelInfo>? Models);

sealed record OllamaModelInfo(string Name);

sealed record OllamaRequest(string Model, bool Stream, IReadOnlyList<ChatMessage> Messages);

sealed record OllamaResponse(OllamaMessage? Message);

sealed record OllamaMessage(string? Content);
