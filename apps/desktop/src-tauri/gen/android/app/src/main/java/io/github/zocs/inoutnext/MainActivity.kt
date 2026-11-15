package io.github.zocs.inoutnext

import android.os.Bundle
import android.view.View
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import java.io.File
import kotlin.math.max

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    RuntimeAssets.install(File(filesDir, "runtime-assets"), assets.open("inout-web-index.tsv")) { path ->
      assets.open(path)
    }
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        // The preview has one host screen. Root Back follows Home: onPause
        // stops sharing while keeping the process available for a quick return.
        moveTaskToBack(true)
      }
    })
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
      view.setPadding(bars.left, bars.top, bars.right, max(bars.bottom, keyboard.bottom))
      // This native container owns the clearance; the WebView must not apply it twice.
      WindowInsetsCompat.CONSUMED
    }
    ViewCompat.requestApplyInsets(content)
  }
}
