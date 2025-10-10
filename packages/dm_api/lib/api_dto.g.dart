// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_dto.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmApiGetProductCountResponse _$DmApiGetProductCountResponseFromJson(
  Map<String, dynamic> json,
) => DmApiGetProductCountResponse(user: (json['user'] as num).toInt());

Map<String, dynamic> _$DmApiGetProductCountResponseToJson(
  DmApiGetProductCountResponse instance,
) => <String, dynamic>{'user': instance.user};

DmApiGetPurchasedProductsResponse _$DmApiGetPurchasedProductsResponseFromJson(
  Map<String, dynamic> json,
) => DmApiGetPurchasedProductsResponse(
  inlinedList: (json['inlinedList'] as List<dynamic>)
      .map((e) => DmApiPurchasedProduct.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$DmApiGetPurchasedProductsResponseToJson(
  DmApiGetPurchasedProductsResponse instance,
) => <String, dynamic>{'inlinedList': instance.inlinedList};

DmApiGetProductsResponse _$DmApiGetProductsResponseFromJson(
  Map<String, dynamic> json,
) => DmApiGetProductsResponse(
  works: (json['works'] as List<dynamic>)
      .map((e) => DmApiProduct.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$DmApiGetProductsResponseToJson(
  DmApiGetProductsResponse instance,
) => <String, dynamic>{'works': instance.works};
