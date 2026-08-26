import 'errors.dart';
import 'models.dart';

const protocolVersion = '1';
const schemaRevision = 'gha-indie-worker-0001';

WorkerLease parseWorkerLease(String id, String revision, Map<String, Object?> payload) {
  if (id.trim().isEmpty) {
    throw const InterfaceException('empty_id');
  }
  if (revision.trim().isEmpty) {
    throw const InterfaceException('empty_revision');
  }
  return WorkerLease(id: id, revision: revision, payload: payload);
}

