import 'package:json_annotation/json_annotation.dart';

part 'dm_api_product.g.dart';

@JsonEnum()
enum DmApiLangKind {
  @JsonValue("en_US")
  en,
  @JsonValue("ja_JP")
  ja,
  @JsonValue("ko_KR")
  ko,
  @JsonValue("zh_TW")
  tw,
  @JsonValue("zh_CN")
  cn,
}

@JsonEnum()
enum DmApiAgeKind {
  @JsonValue("all")
  all,
  @JsonValue("r15")
  r15,
  @JsonValue("r18")
  r18,
}

@JsonSerializable()
class DmApiProduct {
  @JsonKey(name: 'workno')
  final String id;
  @JsonKey(name: 'name')
  final Map<DmApiLangKind, String> name;
  @JsonKey(name: 'maker')
  final DmApiMaker maker;
  @JsonKey(name: 'work_type')
  final String workKind;
  @JsonKey(name: 'age_category')
  final DmApiAgeKind ageKind;
  @JsonKey(name: 'genre_ids')
  final List<int> genreIds;
  @JsonKey(name: 'work_files')
  final DmApiProductThumbnail thumbnail;
  @JsonKey(name: 'regist_date')
  final DateTime registeredAt;
  @JsonKey(name: 'sales_date')
  final DateTime? publishedAt;
  @JsonKey(name: 'upgrade_date')
  final DateTime? updatedAt;
  @JsonKey(name: 'tags', defaultValue: [])
  final List<DmApiProductTag> tags;

  DmApiProduct({
    required this.id,
    required this.name,
    required this.maker,
    required this.workKind,
    required this.ageKind,
    required this.genreIds,
    required this.thumbnail,
    required this.registeredAt,
    required this.publishedAt,
    required this.updatedAt,
    required this.tags,
  });

  factory DmApiProduct.fromJson(Map<String, dynamic> json) =>
      _$DmApiProductFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiProductToJson(this);
}

@JsonSerializable()
class DmApiMaker {
  @JsonKey(name: 'id')
  final String id;
  @JsonKey(name: 'name')
  final Map<DmApiLangKind, String> name;

  DmApiMaker({required this.id, required this.name});

  factory DmApiMaker.fromJson(Map<String, dynamic> json) =>
      _$DmApiMakerFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiMakerToJson(this);
}

@JsonSerializable()
class DmApiProductThumbnail {
  @JsonKey(name: 'main')
  final Uri full;
  @JsonKey(name: 'sam')
  final Uri smallSquare;

  DmApiProductThumbnail({required this.full, required this.smallSquare});

  factory DmApiProductThumbnail.fromJson(Map<String, dynamic> json) =>
      _$DmApiProductThumbnailFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiProductThumbnailToJson(this);
}

@JsonSerializable()
class DmApiProductTag {
  @JsonKey(name: 'class')
  final String key;
  @JsonKey(name: 'name')
  final String value;

  DmApiProductTag({required this.key, required this.value});

  factory DmApiProductTag.fromJson(Map<String, dynamic> json) =>
      _$DmApiProductTagFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiProductTagToJson(this);
}

@JsonSerializable()
class DmApiProductSeries {
  @JsonKey(name: 'title_id')
  final String id;
  @JsonKey(name: 'volume_number')
  final int? index;

  DmApiProductSeries({required this.id, required this.index});

  factory DmApiProductSeries.fromJson(Map<String, dynamic> json) =>
      _$DmApiProductSeriesFromJson(json);

  Map<String, dynamic> toJson() => _$DmApiProductSeriesToJson(this);
}
