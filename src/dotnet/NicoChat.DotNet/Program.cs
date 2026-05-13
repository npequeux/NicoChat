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

    try
    {
        var content = await gateway.GetReplyAsync(request.Messages, cancellationToken);
        return Results.Ok(new ChatReply("assistant", content, gateway.Mode, gateway.Model));
    }
    catch (HttpRequestException exception)
    {
        return Results.Problem(
            title: "Local AI backend unavailable",
            detail: exception.Message,
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
    public string Model { get; } = Environment.GetEnvironmentVariable("OLLAMA_MODEL") ?? "llama3.2:1b";
    public string Mode => _useMock ? "mock" : "ollama";

    public async Task<string> GetReplyAsync(IReadOnlyList<ChatMessage> messages, CancellationToken cancellationToken)
    {
        if (_useMock)
        {
            return BuildMockReply(messages);
        }

        _httpClient.Timeout = TimeSpan.FromSeconds(90);

        using var response = await _httpClient.PostAsJsonAsync(
            $"{_ollamaUrl}/api/chat",
            new OllamaRequest(Model, false, messages.Select(message =>
                new ChatMessage(message.Role.ToLowerInvariant(), message.Content)).ToArray()),
            cancellationToken);

        if (!response.IsSuccessStatusCode)
        {
            var body = await response.Content.ReadAsStringAsync(cancellationToken);
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

sealed record ChatRequest(IReadOnlyList<ChatMessage> Messages);

sealed record ChatMessage(string Role, string Content);

sealed record ChatReply(string Role, string Content, string Mode, string Model);

sealed record ErrorResponse(string Error);

sealed record HealthResponse(string Status, string Backend, string Model, string Mode);

sealed record OllamaRequest(string Model, bool Stream, IReadOnlyList<ChatMessage> Messages);

sealed record OllamaResponse(OllamaMessage? Message);

sealed record OllamaMessage(string? Content);
