import 'dart:convert';

class IrCommand {
  const IrCommand({
    required this.id,
    required this.name,
    required this.carrierHz,
    required this.patternUs,
    this.repeatCount = 1,
  });

  final String id;
  final String name;
  final int carrierHz;
  final List<int> patternUs;
  final int repeatCount;

  IrCommand copyWith({
    String? id,
    String? name,
    int? carrierHz,
    List<int>? patternUs,
    int? repeatCount,
  }) => IrCommand(
    id: id ?? this.id,
    name: name ?? this.name,
    carrierHz: carrierHz ?? this.carrierHz,
    patternUs: patternUs ?? this.patternUs,
    repeatCount: repeatCount ?? this.repeatCount,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'carrier_hz': carrierHz,
    'pattern_us': patternUs,
    'repeat_count': repeatCount,
  };

  factory IrCommand.fromJson(Map<String, dynamic> json) => IrCommand(
    id: json['id'] as String? ?? '',
    name: json['name'] as String? ?? '未命名按键',
    carrierHz: (json['carrier_hz'] as num?)?.toInt() ?? 38000,
    patternUs: (json['pattern_us'] as List<dynamic>? ?? const [])
        .whereType<num>()
        .map((value) => value.toInt())
        .toList(growable: false),
    repeatCount: (json['repeat_count'] as num?)?.toInt() ?? 1,
  );
}

class IrRemoteProfile {
  const IrRemoteProfile({
    required this.id,
    required this.name,
    required this.commands,
    this.builtin = false,
  });

  final String id;
  final String name;
  final List<IrCommand> commands;
  final bool builtin;

  IrRemoteProfile copyWith({
    String? id,
    String? name,
    List<IrCommand>? commands,
    bool? builtin,
  }) => IrRemoteProfile(
    id: id ?? this.id,
    name: name ?? this.name,
    commands: commands ?? this.commands,
    builtin: builtin ?? this.builtin,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'commands': commands.map((command) => command.toJson()).toList(),
    'builtin': builtin,
  };

  factory IrRemoteProfile.fromJson(Map<String, dynamic> json) =>
      IrRemoteProfile(
        id: json['id'] as String? ?? '',
        name: json['name'] as String? ?? '自定义遥控器',
        commands: (json['commands'] as List<dynamic>? ?? const [])
            .whereType<Map>()
            .map((item) => IrCommand.fromJson(Map<String, dynamic>.from(item)))
            .where((command) => command.id.isNotEmpty)
            .toList(growable: false),
        builtin: json['builtin'] as bool? ?? false,
      );
}

class IrMacroStep {
  const IrMacroStep({required this.commandId, this.delayAfterMs = 300});

  final String commandId;
  final int delayAfterMs;

  Map<String, dynamic> toJson() => {
    'command_id': commandId,
    'delay_after_ms': delayAfterMs,
  };

  factory IrMacroStep.fromJson(Map<String, dynamic> json) => IrMacroStep(
    commandId: json['command_id'] as String? ?? '',
    delayAfterMs: (json['delay_after_ms'] as num?)?.toInt() ?? 300,
  );
}

class IrMacro {
  const IrMacro({
    required this.id,
    required this.name,
    required this.profileId,
    required this.steps,
  });

  final String id;
  final String name;
  final String profileId;
  final List<IrMacroStep> steps;

  IrMacro copyWith({String? name, List<IrMacroStep>? steps}) => IrMacro(
    id: id,
    name: name ?? this.name,
    profileId: profileId,
    steps: steps ?? this.steps,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'profile_id': profileId,
    'steps': steps.map((step) => step.toJson()).toList(),
  };

  factory IrMacro.fromJson(Map<String, dynamic> json) => IrMacro(
    id: json['id'] as String? ?? '',
    name: json['name'] as String? ?? '自定义流程',
    profileId: json['profile_id'] as String? ?? '',
    steps: (json['steps'] as List<dynamic>? ?? const [])
        .whereType<Map>()
        .map((item) => IrMacroStep.fromJson(Map<String, dynamic>.from(item)))
        .toList(growable: false),
  );
}

String encodeIrProfiles(List<IrRemoteProfile> profiles) =>
    jsonEncode(profiles.map((profile) => profile.toJson()).toList());

String encodeIrMacros(List<IrMacro> macros) =>
    jsonEncode(macros.map((macro) => macro.toJson()).toList());
