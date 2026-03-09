import 'package:flutter/material.dart';
import '../services/api_service.dart';

/// Ingestion screen for submitting unstructured input.
/// Supports pasting chatlogs, freeform text, voice note transcripts, etc.
class IngestScreen extends StatefulWidget {
  const IngestScreen({super.key});

  @override
  State<IngestScreen> createState() => _IngestScreenState();
}

class _IngestScreenState extends State<IngestScreen> {
  final _controller = TextEditingController();
  final _api = ApiService();
  String _inputType = 'freeform_text';
  String _sourceContext = '';
  bool _submitting = false;
  String? _result;

  final _inputTypes = {
    'freeform_text': 'Freeform Text',
    'chatlog_paste': 'Pasted Chatlog',
    'voice_note': 'Voice Note Transcript',
    'screenshot': 'Screenshot Text (OCR)',
  };

  Future<void> _submit() async {
    final content = _controller.text.trim();
    if (content.isEmpty) return;

    setState(() {
      _submitting = true;
      _result = null;
    });

    try {
      final result = await _api.ingest(
        inputType: _inputType,
        content: content,
        sourceContext: _sourceContext.isNotEmpty ? _sourceContext : null,
      );

      final tasks = (result['tasks'] as List?)?.length ?? 0;
      final events = (result['events'] as List?)?.length ?? 0;
      final conflicts = (result['conflicts'] as List?)?.length ?? 0;

      setState(() {
        _result = 'Extracted: $tasks tasks, $events events, $conflicts conflicts';
        _controller.clear();
      });
    } catch (e) {
      setState(() {
        _result = 'Error: $e';
      });
    }

    setState(() => _submitting = false);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Submit Input', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 8),
          const Text('Paste a chatlog, voice note transcript, or any text. '
              'The AI will extract tasks, events, and contradictions.'),
          const SizedBox(height: 16),

          // Input type selector
          DropdownButtonFormField<String>(
            initialValue: _inputType,
            decoration: const InputDecoration(
              labelText: 'Input Type',
              border: OutlineInputBorder(),
            ),
            items: _inputTypes.entries
                .map((e) => DropdownMenuItem(value: e.key, child: Text(e.value)))
                .toList(),
            onChanged: (v) => setState(() => _inputType = v!),
          ),
          const SizedBox(height: 12),

          // Source context
          TextField(
            decoration: const InputDecoration(
              labelText: 'Context (optional)',
              hintText: 'e.g., "Monday standup", "Team Discord #general"',
              border: OutlineInputBorder(),
            ),
            onChanged: (v) => _sourceContext = v,
          ),
          const SizedBox(height: 12),

          // Main content input
          Expanded(
            child: TextField(
              controller: _controller,
              decoration: const InputDecoration(
                labelText: 'Content',
                hintText: 'Paste your chatlog, notes, or any text here...',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
              maxLines: null,
              expands: true,
              textAlignVertical: TextAlignVertical.top,
            ),
          ),
          const SizedBox(height: 12),

          // Result
          if (_result != null)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: _result!.startsWith('Error')
                    ? Colors.red.withAlpha(25)
                    : Colors.green.withAlpha(25),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(_result!),
            ),
          const SizedBox(height: 8),

          // Submit button
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              onPressed: _submitting ? null : _submit,
              icon: _submitting
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2))
                  : const Icon(Icons.upload),
              label: Text(_submitting ? 'Processing...' : 'Submit for Analysis'),
            ),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }
}
