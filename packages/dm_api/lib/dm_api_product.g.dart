// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_api_product.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmApiProduct _$DmApiProductFromJson(Map<String, dynamic> json) => DmApiProduct(
  id: json['workno'] as String,
  name: (json['name'] as Map<String, dynamic>).map(
    (k, e) => MapEntry($enumDecode(_$DmApiLangKindEnumMap, k), e as String),
  ),
  maker: DmApiMaker.fromJson(json['maker'] as Map<String, dynamic>),
  workKind: json['work_type'] as String,
  ageKind: $enumDecode(_$DmApiAgeKindEnumMap, json['age_category']),
  genreIds: (json['genre_ids'] as List<dynamic>)
      .map((e) => (e as num).toInt())
      .toList(),
  thumbnail: DmApiProductThumbnail.fromJson(
    json['work_files'] as Map<String, dynamic>,
  ),
  registeredAt: DateTime.parse(json['regist_date'] as String),
  publishedAt: DateTime.parse(json['sales_date'] as String),
  updatedAt: DateTime.parse(json['upgrade_date'] as String),
  tags: (json['tags'] as List<dynamic>)
      .map((e) => DmApiProductTag.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$DmApiProductToJson(
  DmApiProduct instance,
) => <String, dynamic>{
  'workno': instance.id,
  'name': instance.name.map((k, e) => MapEntry(_$DmApiLangKindEnumMap[k]!, e)),
  'maker': instance.maker,
  'work_type': instance.workKind,
  'age_category': _$DmApiAgeKindEnumMap[instance.ageKind]!,
  'genre_ids': instance.genreIds,
  'work_files': instance.thumbnail,
  'regist_date': instance.registeredAt.toIso8601String(),
  'sales_date': instance.publishedAt.toIso8601String(),
  'upgrade_date': instance.updatedAt.toIso8601String(),
  'tags': instance.tags,
};

const _$DmApiLangKindEnumMap = {
  DmApiLangKind.en: 'en_US',
  DmApiLangKind.ja: 'ja_JP',
  DmApiLangKind.ko: 'ko_KR',
  DmApiLangKind.tw: 'zh_TW',
  DmApiLangKind.cn: 'zh_CN',
};

const _$DmApiAgeKindEnumMap = {
  DmApiAgeKind.all: 'all',
  DmApiAgeKind.r15: 'r15',
  DmApiAgeKind.r18: 'r18',
};

DmApiMaker _$DmApiMakerFromJson(Map<String, dynamic> json) => DmApiMaker(
  id: json['id'] as String,
  name: (json['name'] as Map<String, dynamic>).map(
    (k, e) => MapEntry($enumDecode(_$DmApiLangKindEnumMap, k), e as String),
  ),
);

Map<String, dynamic> _$DmApiMakerToJson(
  DmApiMaker instance,
) => <String, dynamic>{
  'id': instance.id,
  'name': instance.name.map((k, e) => MapEntry(_$DmApiLangKindEnumMap[k]!, e)),
};

DmApiProductThumbnail _$DmApiProductThumbnailFromJson(
  Map<String, dynamic> json,
) => DmApiProductThumbnail(
  full: Uri.parse(json['main'] as String),
  smallSquare: Uri.parse(json['sam'] as String),
);

Map<String, dynamic> _$DmApiProductThumbnailToJson(
  DmApiProductThumbnail instance,
) => <String, dynamic>{
  'main': instance.full.toString(),
  'sam': instance.smallSquare.toString(),
};

DmApiProductTag _$DmApiProductTagFromJson(Map<String, dynamic> json) =>
    DmApiProductTag(
      key: json['class'] as String,
      value: json['name'] as String,
    );

Map<String, dynamic> _$DmApiProductTagToJson(DmApiProductTag instance) =>
    <String, dynamic>{'class': instance.key, 'name': instance.value};

DmApiProductSeries _$DmApiProductSeriesFromJson(Map<String, dynamic> json) =>
    DmApiProductSeries(
      id: json['title_id'] as String,
      index: (json['volume_number'] as num?)?.toInt(),
    );

Map<String, dynamic> _$DmApiProductSeriesToJson(DmApiProductSeries instance) =>
    <String, dynamic>{'title_id': instance.id, 'volume_number': instance.index};
