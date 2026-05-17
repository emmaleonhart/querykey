import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import '../services/api_service.dart';
import '../models/models.dart';

/// Wiki browser screen — pick a page-type, list pages, open one and render
/// its markdown body with flutter_markdown. Wikilink click-through resolves
/// [[target]] / [[pred:target]] via GET /api/links (R17-5).
class WikiScreen extends StatefulWidget {
  const WikiScreen({super.key});

  @override
  State<WikiScreen> createState() => _WikiScreenState();
}

class _WikiScreenState extends State<WikiScreen> {
  final _api = ApiService();

  // Navigation state: simple enum-like discriminated union.
  _WikiView _view = const _PickerView();

  void _goBack() {
    final view = _view;
    if (view is _DetailView) {
      setState(() => _view = _ListPageView(kind: view.kind));
    } else if (view is _ListPageView) {
      setState(() => _view = const _PickerView());
    }
  }

  void _navigate(String kind, String id, String title) {
    setState(() => _view = _DetailView(
          kind: kind,
          summary: WikiPageSummary(id: id, title: title, kind: kind),
        ));
  }

  @override
  Widget build(BuildContext context) {
    final view = _view;
    if (view is _PickerView) {
      return _TypePickerPage(
        onPicked: (kind) => setState(() => _view = _ListPageView(kind: kind)),
      );
    } else if (view is _ListPageView) {
      return _PageListPage(
        api: _api,
        kind: view.kind,
        onBack: _goBack,
        onOpen: (summary) => setState(
            () => _view = _DetailView(kind: view.kind, summary: summary)),
      );
    } else if (view is _DetailView) {
      return _EntityDetailPage(
        api: _api,
        summary: view.summary,
        onBack: _goBack,
        onNavigate: _navigate,
      );
    }
    return const SizedBox.shrink();
  }
}

// ---- View states ----

sealed class _WikiView {
  const _WikiView();
}

class _PickerView extends _WikiView {
  const _PickerView();
}

class _ListPageView extends _WikiView {
  final String kind;
  const _ListPageView({required this.kind});
}

class _DetailView extends _WikiView {
  final String kind;
  final WikiPageSummary summary;
  const _DetailView({required this.kind, required this.summary});
}

// ---- Type picker page ----

class _TypePickerPage extends StatelessWidget {
  final void Function(String kind) onPicked;
  const _TypePickerPage({required this.onPicked});

  @override
  Widget build(BuildContext context) {
    const types = [
      (kind: 'person', label: 'Contacts', icon: Icons.people_outline),
      (kind: 'project', label: 'Projects', icon: Icons.folder_outlined),
      (kind: 'note', label: 'Notes', icon: Icons.notes_outlined),
      (kind: 'event', label: 'Events', icon: Icons.event_outlined),
    ];

    return Scaffold(
      appBar: AppBar(title: const Text('Wiki')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: types
            .map((t) => Card(
                  child: ListTile(
                    leading: Icon(t.icon),
                    title: Text(t.label),
                    trailing: const Icon(Icons.chevron_right),
                    onTap: () => onPicked(t.kind),
                  ),
                ))
            .toList(),
      ),
    );
  }
}

// ---- Page list page ----

class _PageListPage extends StatefulWidget {
  final ApiService api;
  final String kind;
  final VoidCallback onBack;
  final void Function(WikiPageSummary) onOpen;

  const _PageListPage({
    required this.api,
    required this.kind,
    required this.onBack,
    required this.onOpen,
  });

  @override
  State<_PageListPage> createState() => _PageListPageState();
}

class _PageListPageState extends State<_PageListPage> {
  List<WikiPageSummary> _pages = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final pages = await _listForKind();
      setState(() => _pages = pages);
    } catch (e) {
      setState(() => _error = e.toString());
    }
    setState(() => _loading = false);
  }

  Future<List<WikiPageSummary>> _listForKind() async {
    switch (widget.kind) {
      case 'person':
        final persons = await widget.api.listPersons();
        return persons
            .map((p) => WikiPageSummary(id: p.id, title: p.displayName, kind: 'person'))
            .toList();
      case 'project':
        return widget.api.listProjects();
      case 'note':
        return widget.api.listNotes();
      case 'event':
        return widget.api.listEvents();
      default:
        return [];
    }
  }

  String get _kindLabel => switch (widget.kind) {
        'person' => 'Contacts',
        'project' => 'Projects',
        'note' => 'Notes',
        'event' => 'Events',
        _ => widget.kind,
      };

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_kindLabel),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: widget.onBack,
        ),
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) return const Center(child: CircularProgressIndicator());

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.orange),
            const SizedBox(height: 16),
            Text('Could not load $_kindLabel',
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('Server may not be running',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _load,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    if (_pages.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.folder_open,
                size: 64,
                color: Theme.of(context).colorScheme.primary.withAlpha(100)),
            const SizedBox(height: 16),
            Text('No $_kindLabel yet',
                style: Theme.of(context).textTheme.titleLarge),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        padding: const EdgeInsets.all(8),
        itemCount: _pages.length,
        itemBuilder: (context, i) {
          final p = _pages[i];
          return ListTile(
            title: Text(p.title),
            subtitle:
                Text(p.id, style: Theme.of(context).textTheme.bodySmall),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => widget.onOpen(p),
          );
        },
      ),
    );
  }
}

// ---- Entity detail page ----

class _EntityDetailPage extends StatefulWidget {
  final ApiService api;
  final WikiPageSummary summary;
  final VoidCallback onBack;
  final void Function(String kind, String id, String title) onNavigate;

  const _EntityDetailPage({
    required this.api,
    required this.summary,
    required this.onBack,
    required this.onNavigate,
  });

  @override
  State<_EntityDetailPage> createState() => _EntityDetailPageState();
}

class _EntityDetailPageState extends State<_EntityDetailPage> {
  EntityPage? _page;
  List<dynamic> _backlinks = [];
  List<dynamic> _allLinks = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(_EntityDetailPage old) {
    super.didUpdateWidget(old);
    if (old.summary.id != widget.summary.id ||
        old.summary.kind != widget.summary.kind) {
      _load();
    }
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final results = await Future.wait([
        widget.api.getEntity(widget.summary.kind, widget.summary.id),
        widget.api.entityLinks(widget.summary.kind, widget.summary.id),
        widget.api.listLinks(),
      ]);
      setState(() {
        _page = results[0] as EntityPage;
        final linksData = results[1] as Map<String, dynamic>;
        _backlinks = linksData['to'] as List? ?? [];
        _allLinks = results[2] as List;
      });
    } catch (e) {
      setState(() => _error = e.toString());
    }
    setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.summary.title),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: widget.onBack,
        ),
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) return const Center(child: CircularProgressIndicator());

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.orange),
            const SizedBox(height: 16),
            Text('Could not load page',
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _load,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    final page = _page;
    if (page == null) return const SizedBox.shrink();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Markdown body with wikilink click-through (R17-5)
          if (page.body.isNotEmpty)
            _WikiMarkdown(
              data: page.body,
              allLinks: _allLinks,
              onNavigate: widget.onNavigate,
            ),

          // Backlinks section
          if (_backlinks.isNotEmpty) ...[
            const SizedBox(height: 24),
            const Divider(),
            Text(
              'Backlinks',
              style: Theme.of(context).textTheme.titleSmall?.copyWith(
                    color: Theme.of(context).colorScheme.secondary,
                  ),
            ),
            const SizedBox(height: 8),
            ..._backlinks.map((bl) {
              if (bl is! Map) return const SizedBox.shrink();
              final fromKind = bl['from_kind']?.toString() ?? '';
              final fromId = bl['from_id']?.toString() ?? '';
              final pred = bl['predicate']?.toString() ?? 'references';
              return ListTile(
                dense: true,
                leading: const Icon(Icons.link, size: 16),
                title: Text('$fromKind:$fromId'),
                subtitle: Text(pred),
                onTap: () => widget.onNavigate(fromKind, fromId, fromId),
              );
            }),
          ],
        ],
      ),
    );
  }
}

// ---- Wikilink-aware Markdown renderer (R17-5) ----
//
// Pre-processes the markdown body before passing to flutter_markdown:
//   [[Target]] / [[pred:Target]] that resolve → [Target](qkwiki://kind/id)
//   Dangling (unresolved) → *Target* (rendered as emphasis, non-tappable)
// The `qkwiki://` scheme is intercepted in onTapLink.

class _WikiMarkdown extends StatelessWidget {
  final String data;
  final List<dynamic> allLinks;
  final void Function(String kind, String id, String title) onNavigate;

  const _WikiMarkdown({
    required this.data,
    required this.allLinks,
    required this.onNavigate,
  });

  /// slug(): lowercase, non-alphanumeric runs → single `-`, trimmed.
  /// Matches the server-side slug() function in vault/mod.rs.
  static String _slug(String s) {
    final buf = StringBuffer();
    bool dash = false;
    for (final rune in s.toLowerCase().trim().runes) {
      final ch = String.fromCharCode(rune);
      if (RegExp(r'[a-z0-9]').hasMatch(ch)) {
        buf.write(ch);
        dash = false;
      } else if (buf.isNotEmpty && !dash) {
        buf.write('-');
        dash = true;
      }
    }
    var result = buf.toString();
    while (result.endsWith('-')) {
      result = result.substring(0, result.length - 1);
    }
    return result;
  }

  /// Resolve [[raw]] against the link graph. Returns (kind, id) if a
  /// resolved edge's to_label / to_id matches; null for dangling.
  (String, String)? _resolve(String raw) {
    final target = raw.contains(':') ? raw.split(':').skip(1).join(':') : raw;
    final targetSlug = _slug(target.trim());
    for (final link in allLinks) {
      if (link is! Map) continue;
      if (link['resolved'] != true) continue;
      final label = link['to_label']?.toString() ?? '';
      final toId = link['to_id']?.toString() ?? '';
      final toKind = link['to_kind']?.toString() ?? '';
      if (_slug(label) == targetSlug || _slug(toId) == targetSlug) {
        return (toKind, toId);
      }
    }
    return null;
  }

  /// Replace [[...]] in the markdown with tappable links or dimmed text.
  String _preprocess(String input) {
    return input.replaceAllMapped(
      RegExp(r'\[\[([^\]]+)\]\]'),
      (m) {
        final raw = m.group(1) ?? '';
        final resolved = _resolve(raw);
        if (resolved != null) {
          final (kind, id) = resolved;
          return '[$raw](qkwiki://$kind/$id)';
        }
        return '*$raw*'; // dangling → emphasis, non-tappable
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return MarkdownBody(
      data: _preprocess(data),
      onTapLink: (text, href, title) {
        if (href == null) return;
        final uri = Uri.tryParse(href);
        if (uri == null || uri.scheme != 'qkwiki') return;
        final kind = uri.host;
        final id = uri.pathSegments.isNotEmpty ? uri.pathSegments.first : '';
        if (kind.isNotEmpty && id.isNotEmpty) {
          onNavigate(kind, id, text);
        }
      },
    );
  }
}
