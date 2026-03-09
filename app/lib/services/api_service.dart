import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import '../models/models.dart';

/// REST API client for the Secretarybird server.
class ApiService {
  final String baseUrl;
  final http.Client _client;

  ApiService({this.baseUrl = 'http://127.0.0.1:8000'})
      : _client = http.Client();

  // --- Health ---

  Future<Map<String, dynamic>> health() async {
    final resp = await _get('/health');
    return resp;
  }

  Future<Map<String, dynamic>> systemStatus() async {
    return _get('/api/status');
  }

  Future<Map<String, dynamic>> openclawStatus() async {
    return _get('/api/openclaw/status');
  }

  // --- Ingestion ---

  Future<Map<String, dynamic>> ingest({
    required String inputType,
    required String content,
    String submittedBy = 'app',
    String? sourceContext,
  }) async {
    return _post('/api/ingest', {
      'input_type': inputType,
      'content': content,
      'submitted_by': submittedBy,
      if (sourceContext != null) 'source_context': sourceContext,
    });
  }

  // --- Persons ---

  Future<List<Person>> listPersons() async {
    final data = await _get('/api/persons');
    final list = data['persons'] as List? ?? [];
    return list.map((p) => Person.fromJson(p)).toList();
  }

  Future<Person> createPerson(Person person) async {
    final data = await _post('/api/persons', person.toJson());
    return Person.fromJson(data);
  }

  Future<List<Task>> personTasks(String personId) async {
    final data = await _get('/api/persons/$personId/tasks');
    final list = data['tasks'] as List? ?? [];
    return list.map((t) => Task.fromJson(t)).toList();
  }

  // --- Tasks ---

  Future<List<dynamic>> listTasks() async {
    final data = await _get('/api/tasks');
    return data['tasks'] as List? ?? [];
  }

  Future<Task> createTask(Task task) async {
    final data = await _post('/api/tasks', {
      'title': task.title,
      'description': task.description,
      'assigned_to': task.assignedTo,
      'assigned_by': task.assignedBy,
    });
    return Task.fromJson(data);
  }

  Future<void> updateTaskStatus(String taskId, String status) async {
    await _patch('/api/tasks/$taskId', {'status': status});
  }

  // --- Conflicts ---

  Future<List<Conflict>> listConflicts() async {
    final data = await _get('/api/conflicts');
    final list = data['conflicts'] as List? ?? [];
    return list.map((c) => Conflict.fromJson(c)).toList();
  }

  Future<void> resolveConflict(
      String id, String resolution, String resolvedBy) async {
    await _post('/api/conflicts/$id/resolve', {
      'resolution': resolution,
      'resolved_by': resolvedBy,
    });
  }

  // --- Open Questions ---

  Future<List<dynamic>> listQuestions() async {
    final data = await _get('/api/questions');
    return data['questions'] as List? ?? [];
  }

  // --- OpenClaw Management ---

  Future<void> killOpenClaw() async {
    await _post('/api/openclaw/kill', {});
  }

  Future<Map<String, dynamic>> restartOpenClaw() async {
    return _post('/api/openclaw/restart', {});
  }

  // --- Graph ---

  Future<Map<String, dynamic>> graphQuery(String sparql) async {
    return _post('/api/graph/query', {'query': sparql});
  }

  // --- HTTP helpers ---

  Future<Map<String, dynamic>> _get(String path) async {
    try {
      final resp = await _client.get(Uri.parse('$baseUrl$path'));
      if (resp.statusCode >= 400) {
        throw ApiException(resp.statusCode, resp.body);
      }
      return jsonDecode(resp.body);
    } catch (e) {
      if (e is ApiException) rethrow;
      debugPrint('[api] GET $path failed: $e');
      rethrow;
    }
  }

  Future<Map<String, dynamic>> _post(
      String path, Map<String, dynamic> body) async {
    try {
      final resp = await _client.post(
        Uri.parse('$baseUrl$path'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );
      if (resp.statusCode >= 400) {
        throw ApiException(resp.statusCode, resp.body);
      }
      return jsonDecode(resp.body);
    } catch (e) {
      if (e is ApiException) rethrow;
      debugPrint('[api] POST $path failed: $e');
      rethrow;
    }
  }

  Future<Map<String, dynamic>> _patch(
      String path, Map<String, dynamic> body) async {
    try {
      final resp = await _client.patch(
        Uri.parse('$baseUrl$path'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );
      if (resp.statusCode >= 400) {
        throw ApiException(resp.statusCode, resp.body);
      }
      return jsonDecode(resp.body);
    } catch (e) {
      if (e is ApiException) rethrow;
      debugPrint('[api] PATCH $path failed: $e');
      rethrow;
    }
  }

  void dispose() {
    _client.close();
  }
}

class ApiException implements Exception {
  final int statusCode;
  final String body;
  ApiException(this.statusCode, this.body);

  @override
  String toString() => 'ApiException($statusCode): $body';
}
