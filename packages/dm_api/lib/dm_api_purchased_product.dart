import 'package:json_annotation/json_annotation.dart';

part 'dm_api_purchased_product.g.dart';

@JsonSerializable()
class DmApiPurchasedProduct {
  @JsonKey(name: 'workno')
  final String id;
  @JsonKey(name: 'sales_date')
  final DateTime purchasedAt;

  DmApiPurchasedProduct({required this.id, required this.purchasedAt});

  factory DmApiPurchasedProduct.fromJson(Map<String, dynamic> json) =>
      _$DmApiPurchasedProductFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiPurchasedProductToJson(this);
}
