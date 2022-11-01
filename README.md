# dlsite-manager

This application manages your DLsite products.

![main-image](./docs/img-1.png)

## Key Features

- Manage multiple accounts at once.
- List, search and download your products.

## Account management

You can register many accounts as you want. Products from each account are merged before being listed.

![account-management-image-1](./docs/img-am-1.png)

Don't forget to test your account!

<img src="./docs/img-am-2.png" alt="account-management-image-2" width="300">

## Product download

You can download products by simply clicking the `Download` button. It will show you the progress. The downloading path can be configured via `Setting > Open Settings` menu.

![product-download-image-1](./docs/img-dl-1.png)

After it's done, you can open the saved directory.

![product-download-image-2](./docs/img-dl-2.png)

The saved products are automatically decompressed and cleaned-up if applicable.

### Platform-specific behavior of automatic extraction

Large-sized products (over 1GiB) are shipped in [SFX(Self-extracting archive)](https://en.wikipedia.org/wiki/Self-extracting_archive) format. Since it's a [PE executable](https://en.wikipedia.org/wiki/Portable_Executable), decompression of this format is only can be happened in Windows.
