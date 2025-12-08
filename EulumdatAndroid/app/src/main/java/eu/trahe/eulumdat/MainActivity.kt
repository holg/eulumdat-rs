package eu.trahe.eulumdat

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import eu.trahe.eulumdat.ui.EulumdatApp
import eu.trahe.eulumdat.ui.theme.EulumdatTheme

class MainActivity : ComponentActivity() {

    private var currentFileUri by mutableStateOf<Uri?>(null)
    private var currentFileName by mutableStateOf<String?>(null)

    private val openFileLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        uri?.let { loadFile(it) }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Handle intent if opened via file
        handleIntent(intent)

        setContent {
            EulumdatTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    EulumdatApp(
                        fileUri = currentFileUri,
                        fileName = currentFileName,
                        onOpenFile = { openFile() },
                        onClearFile = { clearFile() }
                    )
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        intent?.data?.let { uri ->
            loadFile(uri)
        }
    }

    private fun openFile() {
        openFileLauncher.launch(arrayOf("*/*"))
    }

    private fun loadFile(uri: Uri) {
        // Get persistent read permission
        try {
            contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION
            )
        } catch (e: SecurityException) {
            // Some URIs don't support persistent permissions
        }

        currentFileUri = uri
        currentFileName = uri.lastPathSegment?.substringAfterLast('/') ?: "Unknown"
    }

    private fun clearFile() {
        currentFileUri = null
        currentFileName = null
    }
}
