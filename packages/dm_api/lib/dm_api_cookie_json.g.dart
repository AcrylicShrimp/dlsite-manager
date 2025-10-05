// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_api_cookie_json.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmApiCookieJson _$DmApiCookieJsonFromJson(Map<String, dynamic> json) =>
    DmApiCookieJson(
      cookies: (json['cookies'] as List<dynamic>)
          .map((e) => e as String)
          .toList(),
    );

Map<String, dynamic> _$DmApiCookieJsonToJson(DmApiCookieJson instance) =>
    <String, dynamic>{'cookies': instance.cookies};
