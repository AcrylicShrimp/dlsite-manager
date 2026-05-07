import 'package:dm_api/dm_api_product.dart';
import "package:fluent_ui/fluent_ui.dart";

class ProductCard extends StatelessWidget {
  final DmApiProduct product;

  const ProductCard({super.key, required this.product});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 100,
              height: 100,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(8.0),
                child: Image.network(
                  product.thumbnail.smallSquare.toString(),
                  fit: BoxFit.cover,
                  loadingBuilder: (context, child, loadingProgress) {
                    if (loadingProgress == null) return child;
                    return const Center(child: ProgressRing());
                  },
                  errorBuilder: (context, error, stackTrace) {
                    return const Icon(FluentIcons.error);
                  },
                ),
              ),
            ),

            const SizedBox(width: 12.0),

            Expanded(
              child: SelectionArea(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      product.name[DmApiLangKind.ko] ??
                          product.name[DmApiLangKind.en] ??
                          product.name[DmApiLangKind.ja] ??
                          "",
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        fontSize: 16,
                        color: FluentTheme.of(context).typography.title?.color,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),

                    const SizedBox(height: 4.0),

                    Text(
                      product.maker.name[DmApiLangKind.ko] ??
                          product.maker.name[DmApiLangKind.en] ??
                          product.maker.name[DmApiLangKind.ja] ??
                          "",
                      style: TextStyle(
                        fontSize: 12,
                        color: FluentTheme.of(
                          context,
                        ).typography.subtitle?.color,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),

                    const SizedBox(height: 16.0),

                    Wrap(
                      spacing: 6.0,
                      runSpacing: 4.0,
                      // ...
                      children: product.tags
                          .map(
                            (tag) => Container(
                              // 내부 여백 (padding)
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8.0,
                                vertical: 4.0,
                              ),
                              // 모양과 색상 (decoration)
                              decoration: BoxDecoration(
                                // Fluent UI의 테마 색상을 사용하는 것이 더 일관성 있습니다.
                                color: FluentTheme.of(
                                  context,
                                ).accentColor.lightest,
                                // 모서리를 완전히 둥글게 만들어 '알약' 모양을 냅니다.
                                borderRadius: BorderRadius.circular(100),
                              ),
                              child: Text(
                                tag.value,
                                style: TextStyle(
                                  fontSize: 11,
                                  color: FluentTheme.of(
                                    context,
                                  ).accentColor.darker,
                                ),
                              ),
                            ),
                          )
                          .toList(),
                      // ...
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
