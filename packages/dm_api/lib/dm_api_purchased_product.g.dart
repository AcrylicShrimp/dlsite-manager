// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_api_purchased_product.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmApiPurchasedProduct _$DmApiPurchasedProductFromJson(
  Map<String, dynamic> json,
) => DmApiPurchasedProduct(
  id: json['workno'] as String,
  purchasedAt: DateTime.parse(json['sales_date'] as String),
);

Map<String, dynamic> _$DmApiPurchasedProductToJson(
  DmApiPurchasedProduct instance,
) => <String, dynamic>{
  'workno': instance.id,
  'sales_date': instance.purchasedAt.toIso8601String(),
};
