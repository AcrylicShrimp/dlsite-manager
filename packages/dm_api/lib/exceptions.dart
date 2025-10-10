import 'package:json_annotation/json_annotation.dart';

class DmApiXsrfTokenNotFoundException implements Exception {
  @override
  String toString() {
    return 'XSRF-TOKEN not found in cookie jar';
  }
}

class DmApiLocationNotFoundException implements Exception {
  final Uri uri;

  DmApiLocationNotFoundException(this.uri);

  @override
  String toString() {
    return '"Location" not found in response headers of $uri';
  }
}

class DmApiCredentialsIncorrectException implements Exception {
  @override
  String toString() {
    return 'credentials are incorrect';
  }
}

class DmApiNotAuthorizedException implements Exception {
  @override
  String toString() {
    return 'not authorized';
  }
}

class DmApiUnexpectedApiResponse implements Exception {
  final String method;
  final String uri;
  final Object? body;
  final CheckedFromJsonException reason;

  DmApiUnexpectedApiResponse(this.method, this.uri, this.body, this.reason);

  @override
  String toString() {
    return 'the API does not respond as expected:\nmethod: $method\nuri: $uri\nbody: $body\nreason: $reason';
  }
}
