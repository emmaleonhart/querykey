// Data models matching the Go server's models package.
// Every entity from docs/data-model.md is represented here.

class Person {
  final String id;
  final String displayName;
  final List<Handle> handles;
  final String? role;
  final List<String> contactCascade;
  final DateTime createdAt;

  Person({
    required this.id,
    required this.displayName,
    this.handles = const [],
    this.role,
    this.contactCascade = const ['app', 'discord'],
    DateTime? createdAt,
  }) : createdAt = createdAt ?? DateTime.now();

  factory Person.fromJson(Map<String, dynamic> json) => Person(
        id: json['id'] ?? '',
        displayName: json['display_name'] ?? '',
        handles: (json['handles'] as List?)
                ?.map((h) => Handle.fromJson(h))
                .toList() ??
            [],
        role: json['role'],
        contactCascade: (json['contact_cascade'] as List?)
                ?.map((e) => e.toString())
                .toList() ??
            ['app', 'discord'],
        createdAt: json['created_at'] != null
            ? DateTime.parse(json['created_at'])
            : DateTime.now(),
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'display_name': displayName,
        'handles': handles.map((h) => h.toJson()).toList(),
        if (role != null) 'role': role,
        'contact_cascade': contactCascade,
      };
}

class Handle {
  final String platform;
  final String identifier;

  Handle({required this.platform, required this.identifier});

  factory Handle.fromJson(Map<String, dynamic> json) => Handle(
        platform: json['platform'] ?? '',
        identifier: json['identifier'] ?? '',
      );

  Map<String, dynamic> toJson() => {
        'platform': platform,
        'identifier': identifier,
      };
}

enum TaskStatus { extracted, confirmed, inProgress, done, disputed }

class Task {
  final String id;
  final String title;
  final String description;
  final TaskStatus status;
  final String? assignedTo;
  final String? assignedBy;
  final DateTime? deadline;
  final double confidence;
  final double ambiguityScore;
  final DateTime createdAt;
  final DateTime updatedAt;

  Task({
    required this.id,
    required this.title,
    this.description = '',
    this.status = TaskStatus.extracted,
    this.assignedTo,
    this.assignedBy,
    this.deadline,
    this.confidence = 0.0,
    this.ambiguityScore = 0.0,
    DateTime? createdAt,
    DateTime? updatedAt,
  })  : createdAt = createdAt ?? DateTime.now(),
        updatedAt = updatedAt ?? DateTime.now();

  factory Task.fromJson(Map<String, dynamic> json) => Task(
        id: json['id'] ?? '',
        title: json['title'] ?? '',
        description: json['description'] ?? '',
        status: _parseStatus(json['status']),
        assignedTo: json['assigned_to'],
        assignedBy: json['assigned_by'],
        deadline: json['deadline'] != null
            ? DateTime.tryParse(json['deadline'])
            : null,
        confidence: (json['confidence'] ?? 0).toDouble(),
        ambiguityScore: (json['ambiguity_score'] ?? 0).toDouble(),
        createdAt: json['created_at'] != null
            ? DateTime.parse(json['created_at'])
            : DateTime.now(),
        updatedAt: json['updated_at'] != null
            ? DateTime.parse(json['updated_at'])
            : DateTime.now(),
      );

  static TaskStatus _parseStatus(String? s) => switch (s) {
        'confirmed' => TaskStatus.confirmed,
        'in_progress' => TaskStatus.inProgress,
        'done' => TaskStatus.done,
        'disputed' => TaskStatus.disputed,
        _ => TaskStatus.extracted,
      };

  String get statusLabel => switch (status) {
        TaskStatus.extracted => 'Extracted',
        TaskStatus.confirmed => 'Confirmed',
        TaskStatus.inProgress => 'In Progress',
        TaskStatus.done => 'Done',
        TaskStatus.disputed => 'Disputed',
      };
}

class Conflict {
  final String id;
  final String type;
  final String explanation;
  final String resolution;
  final DateTime createdAt;

  Conflict({
    required this.id,
    required this.type,
    required this.explanation,
    this.resolution = 'unresolved',
    DateTime? createdAt,
  }) : createdAt = createdAt ?? DateTime.now();

  factory Conflict.fromJson(Map<String, dynamic> json) => Conflict(
        id: json['id'] ?? '',
        type: json['type'] ?? '',
        explanation: json['explanation'] ?? '',
        resolution: json['resolution'] ?? 'unresolved',
        createdAt: json['created_at'] != null
            ? DateTime.parse(json['created_at'])
            : DateTime.now(),
      );
}

class OpenQuestion {
  final String id;
  final String target;
  final String question;
  final String context;
  final String urgency;
  final String status;
  final DateTime createdAt;

  OpenQuestion({
    required this.id,
    required this.target,
    required this.question,
    this.context = '',
    this.urgency = 'whenever',
    this.status = 'open',
    DateTime? createdAt,
  }) : createdAt = createdAt ?? DateTime.now();

  factory OpenQuestion.fromJson(Map<String, dynamic> json) => OpenQuestion(
        id: json['id'] ?? '',
        target: json['target'] ?? '',
        question: json['question'] ?? '',
        context: json['context'] ?? '',
        urgency: json['urgency'] ?? 'whenever',
        status: json['status'] ?? 'open',
        createdAt: json['created_at'] != null
            ? DateTime.parse(json['created_at'])
            : DateTime.now(),
      );
}

/// WebSocket message envelope matching the Go server's WSMessage.
class WSMessage {
  final String type;
  final String? content;
  final dynamic data;

  WSMessage({required this.type, this.content, this.data});

  factory WSMessage.fromJson(Map<String, dynamic> json) => WSMessage(
        type: json['type'] ?? '',
        content: json['content'],
        data: json['data'],
      );
}
