package dev.np.probe;

import android.app.Activity;
import android.os.Bundle;
import android.util.Log;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Proxy;
import java.net.URL;

/** M0 probe: fetches HTTPS URLs through the proxy (explicit java.net.Proxy) and logs results to logcat tag NPPROBE. */
public class MainActivity extends Activity {
    @Override protected void onCreate(Bundle b) {
        super.onCreate(b);
        final String proxyHost = getIntent().getStringExtra("proxyHost") != null ? getIntent().getStringExtra("proxyHost") : "10.0.2.2";
        final int proxyPort = getIntent().getIntExtra("proxyPort", 8899);
        final String base = getIntent().getStringExtra("base") != null ? getIntent().getStringExtra("base") : "https://localhost:9443";
        new Thread(() -> {
            String pkg = getPackageName();
            for (String path : new String[] {"/small", "/__np_mock"}) {
                try {
                    HttpURLConnection c = (HttpURLConnection) new URL(base + path)
                        .openConnection(new Proxy(Proxy.Type.HTTP, new InetSocketAddress(proxyHost, proxyPort)));
                    c.setConnectTimeout(8000); c.setReadTimeout(8000);
                    int code = c.getResponseCode();
                    InputStream in = c.getInputStream(); int n = 0; while (in.read() != -1) n++;
                    Log.i("NPPROBE", pkg + " " + path + " OK status=" + code + " bytes=" + n + " mock=" + c.getHeaderField("x-np-mock"));
                } catch (Throwable t) {
                    Log.i("NPPROBE", pkg + " " + path + " FAIL " + t.getClass().getName() + ": " + t.getMessage());
                }
            }
            Log.i("NPPROBE", pkg + " DONE");
            runOnUiThread(this::finish);
        }).start();
    }
}
