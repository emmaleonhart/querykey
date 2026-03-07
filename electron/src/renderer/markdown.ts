/**
 * Lightweight Markdown renderer for chat messages.
 * Handles code blocks, inline code, bold, italic, headings, lists, and line breaks.
 */

export function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

export function renderMarkdown(text: string): string {
  if (!text) return '';

  let html = escapeHtml(text);

  // Code blocks: ```...```
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (_match, lang: string, code: string) => {
    return `<pre><code class="lang-${lang}">${code.trim()}</code></pre>`;
  });

  // Inline code: `...`
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

  // Bold: **...**
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');

  // Italic: *...*
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');

  // Headings: ### ...
  html = html.replace(/^### (.+)$/gm, '<strong style="font-size:14px;">$1</strong>');
  html = html.replace(/^## (.+)$/gm, '<strong style="font-size:15px;">$1</strong>');

  // Unordered lists: - item
  html = html.replace(/^- (.+)$/gm, '&bull; $1');

  // Line breaks
  html = html.replace(/\n/g, '<br/>');

  return html;
}
