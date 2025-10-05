import 'package:dio/dio.dart';
import 'package:dio_cookie_manager/dio_cookie_manager.dart';
import 'package:dm_api/dm_api_cookie_jar.dart';
import 'package:dm_api/exceptions.dart';

class DmApi {
  final Dio dio;
  final DmApiCookieJar cookieJar;

  DmApi(this.dio, this.cookieJar) {
    dio.interceptors.add(CookieManager(cookieJar.cookieJar));
  }

  Future<void> login(String username, String password) async {
    await dio.get(
      "https://login.dlsite.com/login",
      queryParameters: {"user": "self"},
    );

    final xsrfToken = await cookieJar.findCookieByName(
      Uri.parse("https://login.dlsite.com/login"),
      "XSRF-TOKEN",
    );

    if (xsrfToken == null) {
      throw DmApiXsrfTokenNotFoundException();
    }

    await dio.post(
      "https://login.dlsite.com/login",
      data: {
        "login_id": username,
        "password": password,
        "_token": xsrfToken.value,
      },
      options: Options(
        contentType: "application/x-www-form-urlencoded",
        validateStatus: (status) => status == 302,
        followRedirects: false,
      ),
    );

    final loginRes = await dio.get(
      "https://login.dlsite.com/login",
      options: Options(
        validateStatus: (status) => status == 200 || status == 302,
        followRedirects: false,
      ),
    );
    final loginResBody = loginRes.data.toString();

    if (loginResBody.contains("ログインIDかパスワードが間違っています。")) {
      throw DmApiCredentialsIncorrectException();
    }

    final skipRes = await dio.get(
      "https://www.dlsite.com/home/login/=/skip_register/1",
      options: Options(
        validateStatus: (status) => status == 302,
        followRedirects: false,
      ),
    );
    final oAuthStartLocation = skipRes.headers.value("location");

    if (oAuthStartLocation == null) {
      throw DmApiLocationNotFoundException(skipRes.realUri);
    }

    final oAuthStartRes = await dio.get(
      oAuthStartLocation,
      options: Options(
        validateStatus: (status) => status == 302,
        followRedirects: false,
      ),
    );
    final oAuthRequestLocation = oAuthStartRes.headers.value("location");

    if (oAuthRequestLocation == null) {
      throw DmApiLocationNotFoundException(oAuthStartRes.realUri);
    }

    await dio.get(
      oAuthRequestLocation,
      options: Options(
        validateStatus: (status) => status == 302,
        followRedirects: false,
      ),
    );

    await dio.get("https://www.dlsite.com/home/login/finish");

    final countRes = await dio.get<Map<String, dynamic>>(
      "https://play.dlsite.com/api/v3/content/count",
      options: Options(
        responseType: ResponseType.json,
        validateStatus: (status) => status == 200 || status == 401,
      ),
    );

    if (countRes.statusCode == 401) {
      throw DmApiCredentialsIncorrectException();
    }

    print(countRes.data);
  }
}
