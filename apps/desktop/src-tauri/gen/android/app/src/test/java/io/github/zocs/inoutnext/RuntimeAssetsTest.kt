package io.github.zocs.inoutnext

import java.io.ByteArrayInputStream
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.security.MessageDigest
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class RuntimeAssetsTest {
    @get:Rule val temporary = TemporaryFolder()

    private val files = mapOf(
        "index.html" to "<!doctype html><title>Original HTTP manager</title>".toByteArray(),
        "_next/static/app.js" to "window.assetFixture = 'original bytes';\n".toByteArray(),
    )

    private fun sha(bytes: ByteArray) = MessageDigest.getInstance("SHA-256")
        .digest(bytes).joinToString("") { "%02x".format(it) }

    private fun index(entries: Map<String, ByteArray>): ByteArrayInputStream {
        val table = entries.toSortedMap().entries.joinToString("") { (name, bytes) ->
            "${sha(bytes)}\t${bytes.size}\t$name\n"
        }.toByteArray()
        return ByteArrayInputStream(
            "INOUTWEB1\t${sha(table)}\t${entries.size}\t${entries.values.sumOf { it.size }}\n"
                .toByteArray() + table
        )
    }

    private fun install(runtime: File, entries: Map<String, ByteArray> = files): File =
        RuntimeAssets.install(runtime, index(entries)) { path ->
            ByteArrayInputStream(entries.getValue(path.removePrefix("web/")))
        }

    @Test fun installsRawBytesWithoutSharingTheApplicationDataAncestor() {
        val appFiles = temporary.newFolder("app-files")
        val share = File(appFiles, "shared").apply { mkdir() }
        val privateFile = File(share, "uploaded.txt").apply { writeText("user data") }
        val web = install(File(appFiles, "runtime-assets"))
        assertEquals(File(appFiles, "runtime-assets/web").canonicalFile, web)
        for ((name, content) in files) assertArrayEquals(content, File(web, name).readBytes())
        assertEquals("user data", privateFile.readText())
        assertFalse(File(web, "uploaded.txt").exists())
        assertEquals(setOf("web"), web.parentFile.list()!!.toSet())
    }

    @Test fun validTreeIsReusedWithoutReadingArchiveResourcesAgain() {
        val runtime = temporary.newFolder("runtime-assets")
        val web = install(runtime)
        File(web, "index.html").setLastModified(1_000L)
        val again = RuntimeAssets.install(runtime, index(files)) { error("Valid resources were recopied") }
        assertEquals(web, again)
        assertEquals(1_000L, File(web, "index.html").lastModified())
    }

    @Test fun corruptCachedFilesAndUnlistedFilesAreReplacedByVerifiedOriginals() {
        val runtime = temporary.newFolder("runtime-assets")
        val web = install(runtime)
        File(web, "_next/static/app.js").writeText("invalid cached JavaScript")
        File(web, "unlisted.txt").writeText("must not become a public resource")
        val repaired = install(runtime)
        assertArrayEquals(files.getValue("_next/static/app.js"), File(repaired, "_next/static/app.js").readBytes())
        assertFalse(File(repaired, "unlisted.txt").exists())
        assertEquals(setOf("web"), runtime.list()!!.toSet())
    }

    @Test fun corruptNewArchivePreservesThePreviousCompletedTree() {
        val runtime = temporary.newFolder("runtime-assets")
        val web = install(runtime)
        val updated = files + ("index.html" to "new valid version".toByteArray())
        assertThrows(IOException::class.java) {
            RuntimeAssets.install(runtime, index(updated)) { path ->
                ByteArrayInputStream(if (path == "web/index.html") "corrupt version".toByteArray()
                    else updated.getValue(path.removePrefix("web/")))
            }
        }
        assertArrayEquals(files.getValue("index.html"), File(web, "index.html").readBytes())
        assertEquals(setOf("web"), runtime.list()!!.toSet())
    }

    @Test fun traversalAndNonportableNamesAreRejectedBeforeOpeningSourceData() {
        val appFiles = temporary.newFolder("app-files")
        val privateFile = File(appFiles, "private.txt").apply { writeText("private") }
        for (name in listOf("../private.txt", "/private.txt", "_next/../private.txt", "back\\slash.js", "table\tname.js")) {
            assertThrows(IllegalArgumentException::class.java) {
                RuntimeAssets.install(File(appFiles, "runtime-assets"), index(files + (name to byteArrayOf(1)))) {
                    error("A rejected index opened a resource")
                }
            }
        }
        assertEquals("private", privateFile.readText())
        assertFalse(File(appFiles, "runtime-assets/web").exists())
    }

    @Test fun repairRemovesASymbolicDirectoryWithoutDeletingItsPrivateTarget() {
        val appFiles = temporary.newFolder("app-files")
        val runtime = File(appFiles, "runtime-assets")
        val web = install(runtime)
        val privateDirectory = File(appFiles, "shared").apply { mkdir() }
        val privateFile = File(privateDirectory, "private.txt").apply { writeText("private target") }
        val alias = File(web, "_next/private-alias")
        Files.createSymbolicLink(alias.toPath(), privateDirectory.toPath())
        val repaired = install(runtime)
        assertFalse(File(repaired, "_next/private-alias").exists())
        assertEquals("private target", privateFile.readText())
    }

    @Test fun symbolicRuntimeRootIsRefusedWithoutWritingItsTarget() {
        val appFiles = temporary.newFolder("app-files")
        val privateDirectory = temporary.newFolder("private")
        val alias = File(appFiles, "runtime-assets")
        Files.createSymbolicLink(alias.toPath(), privateDirectory.toPath())
        assertThrows(IOException::class.java) { install(alias) }
        assertTrue(privateDirectory.list()!!.isEmpty())
        Files.delete(alias.toPath())
    }
}
