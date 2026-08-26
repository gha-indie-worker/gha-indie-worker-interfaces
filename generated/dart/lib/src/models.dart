class Health {
  const Health({required this.ok, required this.service, required this.protocol});
  final bool ok;
  final String service;
  final String protocol;
}

class WorkerLease {
  const WorkerLease({required this.id, required this.revision, required this.payload});
  final String id;
  final String revision;
  final Map<String, Object?> payload;
}

