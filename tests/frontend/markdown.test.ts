import { describe, it, expect } from 'vitest';
import { escapeHtml, renderMarkdown } from '../../electron/src/renderer/markdown';

describe('escapeHtml', () => {
  it('escapes HTML special characters', () => {
    expect(escapeHtml('<script>alert("xss")</script>')).toBe(
      '&lt;script&gt;alert("xss")&lt;/script&gt;',
    );
  });

  it('escapes ampersands', () => {
    expect(escapeHtml('a & b')).toBe('a &amp; b');
  });

  it('returns empty string for empty input', () => {
    expect(escapeHtml('')).toBe('');
  });

  it('passes through plain text unchanged', () => {
    expect(escapeHtml('hello world')).toBe('hello world');
  });
});

describe('renderMarkdown', () => {
  it('returns empty string for empty input', () => {
    expect(renderMarkdown('')).toBe('');
  });

  it('renders bold text', () => {
    const result = renderMarkdown('this is **bold** text');
    expect(result).toContain('<strong>bold</strong>');
  });

  it('renders italic text', () => {
    const result = renderMarkdown('this is *italic* text');
    expect(result).toContain('<em>italic</em>');
  });

  it('renders inline code', () => {
    const result = renderMarkdown('use `console.log`');
    expect(result).toContain('<code>console.log</code>');
  });

  it('renders unordered list items', () => {
    const result = renderMarkdown('- item one\n- item two');
    expect(result).toContain('&bull; item one');
    expect(result).toContain('&bull; item two');
  });

  it('converts newlines to <br/>', () => {
    const result = renderMarkdown('line one\nline two');
    expect(result).toContain('<br/>');
  });

  it('escapes HTML in input', () => {
    const result = renderMarkdown('<b>not bold</b>');
    expect(result).not.toContain('<b>');
    expect(result).toContain('&lt;b&gt;');
  });

  it('renders code blocks', () => {
    const result = renderMarkdown('```js\nconst x = 1;\n```');
    expect(result).toContain('<pre><code');
    expect(result).toContain('const x = 1;');
  });

  it('renders h3 headings', () => {
    const result = renderMarkdown('### My Heading');
    expect(result).toContain('My Heading');
    expect(result).toContain('<strong');
  });
});
