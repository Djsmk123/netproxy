#!/bin/zsh
# Builds dev.np.probe.{strict,nsc} APKs with raw SDK tools (no Gradle). Output: scripts/android/out/*.apk
set -e
HERE=${0:A:h}; SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}; BT=$SDK/build-tools/36.0.0; AJ=$SDK/platforms/android-36/android.jar
OUT=$HERE/out; rm -rf $OUT; mkdir -p $OUT/classes $OUT/dex
[ -f $OUT/../debug.keystore ] || keytool -genkeypair -keystore $HERE/debug.keystore -storepass android -keypass android -alias np -keyalg RSA -keysize 2048 -validity 3650 -dname "CN=np-probe" >/dev/null 2>&1
javac --release 8 -cp $AJ -d $OUT/classes $HERE/src/dev/np/probe/MainActivity.java 2>&1 | grep -v "^Note\|warning" || true
$BT/d8 --lib $AJ --output $OUT/dex $OUT/classes/dev/np/probe/*.class
for V in strict nsc; do
  W=$OUT/$V; mkdir -p $W/res/xml
  NSC=""; [ $V = nsc ] && { NSC='android:networkSecurityConfig="@xml/network_security_config"'
    cat > $W/res/xml/network_security_config.xml <<'XML'
<network-security-config><base-config><trust-anchors><certificates src="system"/><certificates src="user"/></trust-anchors></base-config></network-security-config>
XML
  }
  cat > $W/AndroidManifest.xml <<XML
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.np.probe.$V">
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="np-$V" android:usesCleartextTraffic="false" $NSC>
    <activity android:name="dev.np.probe.MainActivity" android:exported="true">
      <intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>
    </activity>
  </application>
</manifest>
XML
  RES=""; if [ $V = nsc ]; then $BT/aapt2 compile --dir $W/res -o $W/res.zip; RES=$W/res.zip; fi
  $BT/aapt2 link -o $W/base.apk -I $AJ --manifest $W/AndroidManifest.xml --min-sdk-version 26 --target-sdk-version 34 $RES
  (cd $OUT/dex && zip -q $W/base.apk classes.dex)
  $BT/zipalign -f 4 $W/base.apk $W/aligned.apk
  $BT/apksigner sign --ks $HERE/debug.keystore --ks-pass pass:android --out $OUT/probe-$V.apk $W/aligned.apk
done
ls -la $OUT/*.apk
