import 'dart:io';

import 'package:dio/dio.dart';
import 'package:dm_api/dm_api.dart';
import 'package:dm_api/dm_api_cookie_jar.dart';
import 'package:test/test.dart';

void main() {
  test('should login', () async {
    final username = Platform.environment['TEST_USERNAME'];
    final password = Platform.environment['TEST_PASSWORD'];

    if (username == null || password == null) {
      throw Exception('TEST_USERNAME and TEST_PASSWORD must be set');
    }

    final dmApi = DmApi(Dio(), DmApiCookieJar.empty());
    await dmApi.login(username, password);
  });
}
