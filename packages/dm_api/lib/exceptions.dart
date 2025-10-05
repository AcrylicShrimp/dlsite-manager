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

class DmApiFailure implements Exception {
  final String context;

  DmApiFailure(this.context);

  @override
  String toString() {
    return 'the API does not respond as expected: $context';
  }
}
