import 'package:app/product_card.dart';
import 'package:dio/dio.dart';
import 'package:dm_api/exceptions.dart';
import 'package:dm_api/dm_api.dart';
import 'package:dm_api/dm_api_cookie_jar.dart';
import 'package:dm_api/dm_api_product.dart';
import "package:fluent_ui/fluent_ui.dart";

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return FluentApp(title: 'DLsite Manager', home: const MainPage());
  }
}

class MainPage extends StatefulWidget {
  const MainPage({super.key});

  @override
  State<MainPage> createState() => _MainPageState();
}

class _MainPageState extends State<MainPage> {
  final DmApi _dmApi = DmApi(Dio(), DmApiCookieJar.empty());
  String _username = "";
  String _password = "";
  List<DmApiProduct> _products = [];
  bool _isFetching = false;

  Future<void> _fetchProducts() async {
    if (_isFetching) return;

    setState(() {
      _isFetching = true;
      _products = [];
    });

    try {
      for (;;) {
        try {
          final purchasedProducts = await _dmApi.getPurchasedProducts();
          final productIds = purchasedProducts.map((e) => e.id).toList();

          productIds.shuffle();

          final products = await _dmApi.getProducts(productIds.sublist(0, 20));

          setState(() {
            _products = products;
          });

          break;
        } on DmApiNotAuthorizedException {
          try {
            await _dmApi.login(_username, _password);
            continue;
          } catch (e) {
            _showErrorDialog('failed to login', e.toString());
            break;
          }
        }
      }
    } catch (e) {
      _showErrorDialog('failed to fetch products', e.toString());
    } finally {
      setState(() {
        _isFetching = false;
      });
    }
  }

  Future<void> _showErrorDialog(String title, String content) async {
    if (!mounted) return;

    showDialog(
      context: context,
      builder: (context) => ContentDialog(
        title: Text(title),
        content: Text(content),
        actions: [
          Button(
            child: const Text('Close'),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return NavigationView(
      appBar: NavigationAppBar(title: const Text("DLsite Manager")),
      content: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            CommandBar(
              primaryItems: [
                CommandBarButton(
                  onPressed: _isFetching ? null : _fetchProducts,
                  icon: _isFetching
                      ? const ProgressRing()
                      : const Icon(FluentIcons.refresh),
                ),
              ],
            ),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    InfoLabel(
                      label: 'Username',
                      child: TextBox(onChanged: (value) => _username = value),
                    ),
                    const SizedBox(height: 12),
                    InfoLabel(
                      label: 'Password',
                      child: PasswordBox(
                        onChanged: (value) => _password = value,
                      ),
                    ),
                  ],
                ),
              ),
            ),

            const SizedBox(height: 20),

            Expanded(child: _buildProductList()),
          ],
        ),
      ),
    );
  }

  Widget _buildProductList() {
    if (_isFetching) {
      return const Center(child: ProgressRing());
    }

    if (_products.isEmpty) {
      return const Center(
        child: Text(
          'products list is empty.\nplease press the button to fetch the list.',
        ),
      );
    }

    return ListView.builder(
      itemCount: _products.length,
      itemBuilder: (context, index) {
        return ProductCard(product: _products[index]);
      },
    );
  }
}
