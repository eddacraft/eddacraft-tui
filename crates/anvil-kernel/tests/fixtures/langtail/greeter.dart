// LANGTAIL T1 fixture — representative Dart source (Flutter-flavoured).
import 'dart:async';
import 'package:meta/meta.dart';

/// A small greeter service.
class Greeter {
  final String prefix;

  Greeter(this.prefix);

  Future<String> greet(String name) async {
    return '$prefix, $name';
  }

  void _logInternal(String message) {
    print(message);
  }
}

mixin Loggable {
  void log(String line) => print(line);
}

enum Mood { happy, neutral, grumpy }

String topLevelGreeting(String who) => 'hello $who';
