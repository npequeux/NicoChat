"""Tests for static web UI import/export controls."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_web_index_contains_document_import_controls():
    html = (ROOT / "web" / "index.html").read_text(encoding="utf-8")
    assert 'id="importDocumentButton"' in html
    assert 'id="importDocumentInput"' in html


def test_web_app_contains_reply_export_controls():
    js = (ROOT / "web" / "app.js").read_text(encoding="utf-8")
    assert "Export picture" in js
    assert "Export document" in js
    assert "importDocumentToComposer" in js
