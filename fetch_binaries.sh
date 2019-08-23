#!/bin/sh -ex
ALPINE_VERSION=v3.10
ALPINE_MIRROR=http://dl-cdn.alpinelinux.org/alpine/
APK_TOOLS=apk-tools-static-2.10.4-r2.apk
BUSYBOX=busybox-static-1.30.1-r2.apk
ALPINE_KEYS=alpine-keys-2.1-r2.apk

ARCH=${1:-x86_64}

SHA1SUMS_x86_64="\
23a28a47cf4cb0ab42fde4f6e6af20a8e595488a  $APK_TOOLS
de4c07d01f3a014b341345abaf89d5792cb32eb0  $BUSYBOX
0004212c1be4195853b9ea964b191374dfc1c36b  $ALPINE_KEYS"

SHA1SUMS_armhf="\
70cdc74b27393e6bf514e81ef4b596f79c0f44b8  $APK_TOOLS
6e8b46d4671bd5c36a9ae0f04fabff5bc847229f  $BUSYBOX
963b900f31d62814d0a30d41e3596e53766844c5  $ALPINE_KEYS"

FETCH_DIR="alpine/"$ARCH
mkdir -p "$FETCH_DIR" 2>/dev/null || true
cd "$FETCH_DIR"

for pkg in $APK_TOOLS $BUSYBOX $ALPINE_KEYS; do
    wget --no-use-server-timestamp ${ALPINE_MIRROR}${ALPINE_VERSION}/main/$ARCH/$pkg -O $pkg
done

sha1sum $APK_TOOLS
sha1sum $BUSYBOX
sha1sum $ALPINE_KEYS
SUMS="SHA1SUMS_$ARCH"
eval "SUMS=\$$SUMS"
echo "$SUMS" | sha1sum -c -

cd ../..

tar -xOf "$FETCH_DIR/$APK_TOOLS" sbin/apk.static > apk
tar -xOf "$FETCH_DIR/$BUSYBOX" bin/busybox.static > busybox
cp "$FETCH_DIR/$ALPINE_KEYS" alpine-keys.apk

chmod +x apk busybox
