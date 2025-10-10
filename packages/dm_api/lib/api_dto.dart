import 'package:dm_api/dm_api_product.dart';
import 'package:dm_api/dm_api_purchased_product.dart';
import 'package:json_annotation/json_annotation.dart';

part 'api_dto.g.dart';

@JsonSerializable()
class DmApiGetProductCountResponse {
  final int user;

  DmApiGetProductCountResponse({required this.user});

  factory DmApiGetProductCountResponse.fromJson(Map<String, dynamic> json) =>
      _$DmApiGetProductCountResponseFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiGetProductCountResponseToJson(this);
}

@JsonSerializable()
class DmApiGetPurchasedProductsResponse {
  final List<DmApiPurchasedProduct> inlinedList;

  DmApiGetPurchasedProductsResponse({required this.inlinedList});

  factory DmApiGetPurchasedProductsResponse.fromJson(
    Map<String, dynamic> json,
  ) => _$DmApiGetPurchasedProductsResponseFromJson(json);

  Map<String, dynamic> toJson() =>
      _$DmApiGetPurchasedProductsResponseToJson(this);
}

@JsonSerializable()
class DmApiGetProductsResponse {
  final List<DmApiProduct> works;

  DmApiGetProductsResponse({required this.works});

  factory DmApiGetProductsResponse.fromJson(Map<String, dynamic> json) =>
      _$DmApiGetProductsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiGetProductsResponseToJson(this);
}
