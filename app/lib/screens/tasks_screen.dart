import 'package:flutter/material.dart';
import '../services/api_service.dart';

/// Task board screen showing extracted, confirmed, in-progress, and done tasks.
/// Every task shows its confidence score and source.
class TasksScreen extends StatefulWidget {
  const TasksScreen({super.key});

  @override
  State<TasksScreen> createState() => _TasksScreenState();
}

class _TasksScreenState extends State<TasksScreen> {
  final _api = ApiService();
  List<dynamic> _tasks = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadTasks();
  }

  Future<void> _loadTasks() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      _tasks = await _api.listTasks();
    } catch (e) {
      _error = e.toString();
    }
    setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.orange),
            const SizedBox(height: 16),
            Text('Could not load tasks', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('Server may not be running', style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _loadTasks,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    if (_tasks.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.task_alt, size: 64, color: Theme.of(context).colorScheme.primary.withAlpha(100)),
            const SizedBox(height: 16),
            Text('No tasks yet', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 8),
            const Text('Tasks will appear here as the AI extracts them from conversations.'),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _loadTasks,
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: _tasks.length,
        itemBuilder: (context, index) {
          final task = _tasks[index];
          return _TaskCard(task: task);
        },
      ),
    );
  }
}

class _TaskCard extends StatelessWidget {
  final dynamic task;

  const _TaskCard({required this.task});

  @override
  Widget build(BuildContext context) {
    final title = task is Map ? (task['title']?['value'] ?? task['title'] ?? 'Untitled') : 'Untitled';
    final status = task is Map ? (task['status']?['value'] ?? task['status'] ?? 'extracted') : 'extracted';
    final confidence = task is Map ? (task['confidence']?['value'] ?? '') : '';

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: _statusIcon(status.toString()),
        title: Text(title.toString()),
        subtitle: confidence.toString().isNotEmpty
            ? Text('Confidence: ${(double.tryParse(confidence.toString()) ?? 0 * 100).toStringAsFixed(0)}%')
            : null,
        trailing: Chip(
          label: Text(status.toString()),
          backgroundColor: _statusColor(status.toString()),
        ),
      ),
    );
  }

  Widget _statusIcon(String status) {
    return switch (status) {
      'extracted' => const Icon(Icons.auto_awesome, color: Colors.amber),
      'confirmed' => const Icon(Icons.check_circle_outline, color: Colors.blue),
      'in_progress' => const Icon(Icons.play_circle_outline, color: Colors.green),
      'done' => const Icon(Icons.task_alt, color: Colors.grey),
      'disputed' => const Icon(Icons.warning, color: Colors.red),
      _ => const Icon(Icons.circle_outlined),
    };
  }

  Color _statusColor(String status) {
    return switch (status) {
      'extracted' => Colors.amber.withAlpha(50),
      'confirmed' => Colors.blue.withAlpha(50),
      'in_progress' => Colors.green.withAlpha(50),
      'done' => Colors.grey.withAlpha(50),
      'disputed' => Colors.red.withAlpha(50),
      _ => Colors.grey.withAlpha(50),
    };
  }
}
