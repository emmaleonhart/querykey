import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/websocket_service.dart';
import 'screens/profile_screen.dart';
import 'screens/wiki_screen.dart';

void main() {
  runApp(const QueryKeyApp());
}

class QueryKeyApp extends StatelessWidget {
  const QueryKeyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => WebSocketService()..connect(),
      child: MaterialApp(
        title: 'QueryKey',
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
///
/// QueryKey surfaces, deliberately scoped (2026-05-16): just **Profile**
/// (your own card — key/query signal) and **Wiki** (browse the vault +
/// click through `[[wikilinks]]`). Chat / Tasks / Ingest were the old
/// Secretary Bird screens; they are removed until the local agent is
/// actually wired in, so the app only shows what truly works.
class MainShell extends StatefulWidget {
  const MainShell({super.key});

  @override
  State<MainShell> createState() => _MainShellState();
}

class _MainShellState extends State<MainShell> {
  int _selectedIndex = 0;

  static const _destinations = [
    NavigationRailDestination(
      icon: Icon(Icons.person_outline),
      selectedIcon: Icon(Icons.person),
      label: Text('Profile'),
    ),
    NavigationRailDestination(
      icon: Icon(Icons.book_outlined),
      selectedIcon: Icon(Icons.book),
      label: Text('Wiki'),
    ),
  ];

  static const _screens = [
    ProfileScreen(),
    WikiScreen(),
  ];

  /// The QueryKey mark: a "QK" badge (Query / Key — the Q/K of attention).
  /// Replaces the old placeholder `Icons.pets` paw.
  Widget _brandMark(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      children: [
        Container(
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: cs.primary,
            borderRadius: BorderRadius.circular(8),
          ),
          alignment: Alignment.center,
          child: Text(
            'QK',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: cs.onPrimary,
                  fontWeight: FontWeight.bold,
                ),
          ),
        ),
        const SizedBox(height: 4),
        Text('QueryKey', style: Theme.of(context).textTheme.labelSmall),
      ],
    );
  }

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
                child: _brandMark(context),
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
          NavigationDestination(
              icon: Icon(Icons.person_outline),
              selectedIcon: Icon(Icons.person),
              label: 'Profile'),
          NavigationDestination(
              icon: Icon(Icons.book_outlined),
              selectedIcon: Icon(Icons.book),
              label: 'Wiki'),
        ],
      ),
    );
  }
}
