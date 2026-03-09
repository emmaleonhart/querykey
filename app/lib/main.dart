import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/websocket_service.dart';
import 'screens/chat_screen.dart';
import 'screens/tasks_screen.dart';
import 'screens/ingest_screen.dart';

void main() {
  runApp(const SecretarybirdApp());
}

class SecretarybirdApp extends StatelessWidget {
  const SecretarybirdApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => WebSocketService()..connect(),
      child: MaterialApp(
        title: 'Secretarybird',
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(
            seedColor: const Color(0xFF455A64), // blue-grey
            brightness: Brightness.dark,
          ),
          useMaterial3: true,
        ),
        home: const MainShell(),
      ),
    );
  }
}

/// Main navigation shell with sidebar on desktop, bottom nav on mobile.
class MainShell extends StatefulWidget {
  const MainShell({super.key});

  @override
  State<MainShell> createState() => _MainShellState();
}

class _MainShellState extends State<MainShell> {
  int _selectedIndex = 0;

  static const _destinations = [
    NavigationRailDestination(
      icon: Icon(Icons.chat_outlined),
      selectedIcon: Icon(Icons.chat),
      label: Text('Chat'),
    ),
    NavigationRailDestination(
      icon: Icon(Icons.task_outlined),
      selectedIcon: Icon(Icons.task),
      label: Text('Tasks'),
    ),
    NavigationRailDestination(
      icon: Icon(Icons.upload_outlined),
      selectedIcon: Icon(Icons.upload),
      label: Text('Ingest'),
    ),
  ];

  static const _screens = [
    ChatScreen(),
    TasksScreen(),
    IngestScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    final isWide = MediaQuery.of(context).size.width > 600;

    if (isWide) {
      // Desktop layout: sidebar + content
      return Scaffold(
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: _selectedIndex,
              onDestinationSelected: (i) => setState(() => _selectedIndex = i),
              labelType: NavigationRailLabelType.all,
              leading: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  children: [
                    Icon(Icons.pets, size: 32,
                        color: Theme.of(context).colorScheme.primary),
                    const SizedBox(height: 4),
                    Text('Secretarybird',
                        style: Theme.of(context).textTheme.labelSmall),
                  ],
                ),
              ),
              destinations: _destinations,
            ),
            const VerticalDivider(width: 1),
            Expanded(child: _screens[_selectedIndex]),
          ],
        ),
      );
    }

    // Mobile layout: bottom navigation
    return Scaffold(
      body: _screens[_selectedIndex],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (i) => setState(() => _selectedIndex = i),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.chat_outlined), selectedIcon: Icon(Icons.chat), label: 'Chat'),
          NavigationDestination(icon: Icon(Icons.task_outlined), selectedIcon: Icon(Icons.task), label: 'Tasks'),
          NavigationDestination(icon: Icon(Icons.upload_outlined), selectedIcon: Icon(Icons.upload), label: 'Ingest'),
        ],
      ),
    );
  }
}
