import 'package:flutter/material.dart';
import '../services/api_service.dart';
import '../models/models.dart';

/// Profile/Card screen — view and edit your P2P card (key/query signal).
///
/// Displays: bio, Offering (key), Looking-for (query), handle, website,
/// propagation status (24h safety valve: pending + eligible_at).
/// Edit form → PUT /api/card. Buttons: "Draft with agent" (POST /api/card/draft)
/// and "Revert" (POST /api/card/revert).
class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key});

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  final _api = ApiService();

  QkCard? _card;
  CardPropagation? _propagation;
  bool _loading = true;
  String? _error;

  // Edit-mode state
  bool _editing = false;
  final _formKey = GlobalKey<FormState>();
  late TextEditingController _handleCtrl;
  late TextEditingController _nameCtrl;
  late TextEditingController _websiteCtrl;
  late TextEditingController _bioCtrl;
  late TextEditingController _offeringCtrl;
  late TextEditingController _lookingForCtrl;

  @override
  void initState() {
    super.initState();
    _handleCtrl = TextEditingController();
    _nameCtrl = TextEditingController();
    _websiteCtrl = TextEditingController();
    _bioCtrl = TextEditingController();
    _offeringCtrl = TextEditingController();
    _lookingForCtrl = TextEditingController();
    _loadCard();
  }

  @override
  void dispose() {
    _handleCtrl.dispose();
    _nameCtrl.dispose();
    _websiteCtrl.dispose();
    _bioCtrl.dispose();
    _offeringCtrl.dispose();
    _lookingForCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadCard() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await _api.getCard();
      final cardJson = data['card'] as Map<String, dynamic>?;
      final propJson = data['propagation'] as Map<String, dynamic>?;
      setState(() {
        _card = cardJson != null ? QkCard.fromJson(cardJson) : null;
        _propagation = propJson != null ? CardPropagation.fromJson(propJson) : null;
      });
    } catch (e) {
      setState(() => _error = e.toString());
    }
    setState(() => _loading = false);
  }

  void _startEdit() {
    final c = _card;
    if (c != null) {
      _handleCtrl.text = c.handle;
      _nameCtrl.text = c.name;
      _websiteCtrl.text = c.website;
      _bioCtrl.text = c.bio;
      _offeringCtrl.text = c.offering.join('\n');
      _lookingForCtrl.text = c.lookingFor.join('\n');
    }
    setState(() => _editing = true);
  }

  Future<void> _saveCard() async {
    if (!_formKey.currentState!.validate()) return;
    final updated = QkCard(
      handle: _handleCtrl.text.trim(),
      name: _nameCtrl.text.trim(),
      website: _websiteCtrl.text.trim(),
      bio: _bioCtrl.text.trim(),
      offering: _offeringCtrl.text
          .split('\n')
          .map((l) => l.trim())
          .where((l) => l.isNotEmpty)
          .toList(),
      lookingFor: _lookingForCtrl.text
          .split('\n')
          .map((l) => l.trim())
          .where((l) => l.isNotEmpty)
          .toList(),
      updated: DateTime.now(),
    );
    try {
      await _api.putCard(updated);
      setState(() => _editing = false);
      await _loadCard();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Save failed: $e')),
        );
      }
    }
  }

  Future<void> _draftCard() async {
    setState(() => _loading = true);
    try {
      final data = await _api.draftCard();
      final draft = data['draft'] as Map<String, dynamic>?;
      final source = data['source'] as String? ?? 'unknown';
      if (draft != null && mounted) {
        final draftCard = QkCard.fromJson(draft);
        _handleCtrl.text = draftCard.handle;
        _nameCtrl.text = draftCard.name;
        _websiteCtrl.text = draftCard.website;
        _bioCtrl.text = draftCard.bio;
        _offeringCtrl.text = draftCard.offering.join('\n');
        _lookingForCtrl.text = draftCard.lookingFor.join('\n');
        if (!_editing) setState(() => _editing = true);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Draft generated (source: $source). Review and save.')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Draft failed: $e')),
        );
      }
    }
    setState(() => _loading = false);
  }

  Future<void> _revertCard() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Revert card?'),
        content: const Text(
          'This will discard the pending staged edit and restore the last published version. Cannot be undone.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Revert')),
        ],
      ),
    );
    if (confirmed != true) return;
    try {
      await _api.revertCard();
      setState(() => _editing = false);
      await _loadCard();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Revert failed: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.orange),
            const SizedBox(height: 16),
            Text('Could not load card', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('Server may not be running', style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _loadCard,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    return _editing ? _buildEditForm() : _buildViewCard();
  }

  Widget _buildViewCard() {
    final c = _card;
    if (c == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.person_outline, size: 64,
                color: Theme.of(context).colorScheme.primary.withAlpha(100)),
            const SizedBox(height: 16),
            Text('No card yet', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 8),
            const Text('Your card is your key/query signal for the P2P layer.'),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                FilledButton.icon(
                  onPressed: _startEdit,
                  icon: const Icon(Icons.edit),
                  label: const Text('Create card'),
                ),
                const SizedBox(width: 8),
                OutlinedButton.icon(
                  onPressed: _draftCard,
                  icon: const Icon(Icons.auto_awesome),
                  label: const Text('Draft with agent'),
                ),
              ],
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _loadCard,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header row
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        c.name.isNotEmpty ? c.name : c.handle,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                      if (c.handle.isNotEmpty)
                        Text(c.handle,
                            style: Theme.of(context).textTheme.bodySmall),
                    ],
                  ),
                ),
                IconButton(
                  onPressed: _startEdit,
                  icon: const Icon(Icons.edit),
                  tooltip: 'Edit card',
                ),
              ],
            ),
            if (c.website.isNotEmpty) ...[
              const SizedBox(height: 4),
              Text(c.website,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.primary,
                  )),
            ],

            // Bio
            if (c.bio.isNotEmpty) ...[
              const SizedBox(height: 12),
              Text(c.bio, style: Theme.of(context).textTheme.bodyMedium),
            ],

            const SizedBox(height: 16),
            const Divider(),

            // Offering (key)
            _sectionHeader(context, 'Offering (key)'),
            const SizedBox(height: 8),
            if (c.offering.isEmpty)
              Text('—', style: Theme.of(context).textTheme.bodySmall)
            else
              ...c.offering.map((o) => _bulletItem(context, o)),

            const SizedBox(height: 16),

            // Looking for (query)
            _sectionHeader(context, 'Looking for (query)'),
            const SizedBox(height: 8),
            if (c.lookingFor.isEmpty)
              Text('—', style: Theme.of(context).textTheme.bodySmall)
            else
              ...c.lookingFor.map((q) => _bulletItem(context, q)),

            const SizedBox(height: 16),
            const Divider(),

            // Propagation status (24h valve)
            _buildPropagationStatus(),

            const SizedBox(height: 16),

            // Action buttons
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton.icon(
                  onPressed: _draftCard,
                  icon: const Icon(Icons.auto_awesome),
                  label: const Text('Draft with agent'),
                ),
                if (_propagation?.pending == true)
                  OutlinedButton.icon(
                    onPressed: _revertCard,
                    icon: const Icon(Icons.undo),
                    label: const Text('Revert'),
                    style: OutlinedButton.styleFrom(
                      foregroundColor: Theme.of(context).colorScheme.error,
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPropagationStatus() {
    final p = _propagation;
    if (p == null) return const SizedBox.shrink();

    final String statusText;
    final Color statusColor;
    if (p.pending) {
      final eligible = p.eligibleAt;
      if (eligible != null) {
        final remaining = eligible.difference(DateTime.now());
        final h = remaining.inHours;
        final m = remaining.inMinutes % 60;
        statusText = 'Pending — eligible in ${h}h ${m}m (24h safety valve)';
      } else {
        statusText = 'Pending — 24h propagation delay active';
      }
      statusColor = Colors.orange;
    } else if (p.published) {
      statusText = 'Published';
      statusColor = Colors.green;
    } else {
      statusText = 'Not yet published';
      statusColor = Colors.grey;
    }

    return Row(
      children: [
        Icon(Icons.circle, size: 10, color: statusColor),
        const SizedBox(width: 8),
        Expanded(
          child: Text(statusText,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(color: statusColor)),
        ),
      ],
    );
  }

  Widget _buildEditForm() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text('Edit card',
                      style: Theme.of(context).textTheme.headlineSmall),
                ),
                TextButton(
                  onPressed: () => setState(() => _editing = false),
                  child: const Text('Cancel'),
                ),
              ],
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _handleCtrl,
              decoration: const InputDecoration(
                labelText: 'Handle',
                hintText: 'github:yourusername',
                border: OutlineInputBorder(),
              ),
              validator: (v) =>
                  v == null || v.trim().isEmpty ? 'Handle is required' : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _nameCtrl,
              decoration: const InputDecoration(
                labelText: 'Display name',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _websiteCtrl,
              decoration: const InputDecoration(
                labelText: 'Website',
                hintText: 'https://...',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _bioCtrl,
              decoration: const InputDecoration(
                labelText: 'Bio (one line)',
                border: OutlineInputBorder(),
              ),
              maxLines: 2,
            ),
            const SizedBox(height: 16),
            Text('Offering (key)', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text('One item per line — what you offer/provide.',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 8),
            TextFormField(
              controller: _offeringCtrl,
              decoration: const InputDecoration(
                hintText: 'Rust mentoring\nSystems design',
                border: OutlineInputBorder(),
              ),
              maxLines: 4,
            ),
            const SizedBox(height: 16),
            Text('Looking for (query)', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text('One item per line — what you\'re seeking.',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 8),
            TextFormField(
              controller: _lookingForCtrl,
              decoration: const InputDecoration(
                hintText: 'Flutter reviewers\nFunding advice',
                border: OutlineInputBorder(),
              ),
              maxLines: 4,
            ),
            const SizedBox(height: 24),
            Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    onPressed: _saveCard,
                    icon: const Icon(Icons.save),
                    label: const Text('Save card'),
                  ),
                ),
                const SizedBox(width: 8),
                OutlinedButton.icon(
                  onPressed: _draftCard,
                  icon: const Icon(Icons.auto_awesome),
                  label: const Text('Draft with agent'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _sectionHeader(BuildContext context, String title) => Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
      );

  Widget _bulletItem(BuildContext context, String text) => Padding(
        padding: const EdgeInsets.only(bottom: 4),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('• ', style: TextStyle(fontWeight: FontWeight.bold)),
            Expanded(child: Text(text)),
          ],
        ),
      );
}
