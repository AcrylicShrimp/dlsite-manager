import 'package:cookie_jar/cookie_jar.dart';
import 'package:dm_api/dm_api_cookie_json.dart';

final _dlsiteUri = Uri.parse("https://dlsite.com");

class DmApiCookieJar {
  final CookieJar cookieJar;

  DmApiCookieJar._(this.cookieJar);

  DmApiCookieJar.empty() : this._(CookieJar());

  factory DmApiCookieJar.fromJson(Map<String, dynamic> json) {
    final cookieJson = DmApiCookieJson.fromJson(json);
    final cookies = cookieJson.cookies
        .map((cookie) => Cookie.fromSetCookieValue(cookie))
        .toList();

    final cookieJar = CookieJar();
    cookieJar.saveFromResponse(_dlsiteUri, cookies);

    return DmApiCookieJar._(cookieJar);
  }

  Future<Map<String, dynamic>> toJson() async {
    final cookies = await cookieJar.loadForRequest(_dlsiteUri);
    final cookieStrings = cookies.map((cookie) => cookie.toString()).toList();

    return DmApiCookieJson(cookies: cookieStrings).toJson();
  }

  Future<Cookie?> findCookieByName(Uri uri, String name) async {
    final cookies = await cookieJar.loadForRequest(uri);

    for (final cookie in cookies) {
      if (cookie.name == name) {
        return cookie;
      }
    }

    return null;
  }
}
