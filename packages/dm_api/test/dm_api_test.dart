import 'dart:io';

import 'package:dio/dio.dart';
import 'package:dm_api/dm_api.dart';
import 'package:dm_api/dm_api_cookie_jar.dart';
import 'package:dm_api/dm_api_product.dart';
import 'package:dm_api/dm_api_purchased_product.dart';
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

  test("should get product count", () async {
    final username = Platform.environment['TEST_USERNAME'];
    final password = Platform.environment['TEST_PASSWORD'];

    if (username == null || password == null) {
      throw Exception('TEST_USERNAME and TEST_PASSWORD must be set');
    }

    final dmApi = DmApi(Dio(), DmApiCookieJar.empty());
    await dmApi.login(username, password);

    final count = await dmApi.getProductCount();
    expect(count, isA<int>());
  });

  test("should get purchased products", () async {
    final username = Platform.environment['TEST_USERNAME'];
    final password = Platform.environment['TEST_PASSWORD'];

    if (username == null || password == null) {
      throw Exception('TEST_USERNAME and TEST_PASSWORD must be set');
    }

    final dmApi = DmApi(Dio(), DmApiCookieJar.empty());
    await dmApi.login(username, password);

    final purchasedProducts = await dmApi.getPurchasedProducts();
    expect(purchasedProducts, isA<List<DmApiPurchasedProduct>>());
  });

  test("should get products", () async {
    final username = Platform.environment['TEST_USERNAME'];
    final password = Platform.environment['TEST_PASSWORD'];

    if (username == null || password == null) {
      throw Exception('TEST_USERNAME and TEST_PASSWORD must be set');
    }

    final dmApi = DmApi(Dio(), DmApiCookieJar.empty());
    await dmApi.login(username, password);

    final purchasedProducts = await dmApi.getPurchasedProducts();
    final productIds = purchasedProducts.map((e) => e.id).toList();
    final products = await dmApi.getProducts(productIds.sublist(0, 10));
    expect(products, isA<List<DmApiProduct>>());
  });
}
