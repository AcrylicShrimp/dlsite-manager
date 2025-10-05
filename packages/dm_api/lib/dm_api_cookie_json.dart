import 'package:json_annotation/json_annotation.dart';

part 'dm_api_cookie_json.g.dart';

@JsonSerializable()
class DmApiCookieJson {
  final List<String> cookies;

  DmApiCookieJson({required this.cookies});

  factory DmApiCookieJson.fromJson(Map<String, dynamic> json) =>
      _$DmApiCookieJsonFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiCookieJsonToJson(this);
}
