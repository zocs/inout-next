package io.github.zocs.inoutnext

import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest
import java.util.UUID

/** Installs original APK resources into an owned tree separate from shared data. */
object RuntimeAssets {
    private const val MAX_FILES = 10_000
    private const val MAX_BYTES = 128L * 1024 * 1024
    private const val MAX_DEPTH = 128
    private const val MAX_INDEX_BYTES = 2 * 1024 * 1024
    private val resourcePath = Regex("[A-Za-z0-9_.@/-]+")
    private val checksum = Regex("[0-9a-f]{64}")

    private data class Entry(val path: String, val bytes: Long, val sha256: String)
    private data class Index(val entries: List<Entry>, val directories: Set<String>)

    @Synchronized
    fun install(runtimeParent: File, indexInput: InputStream, openAsset: (String) -> InputStream): File {
        val index = readIndex(indexInput)
        val parent = runtimeParent.parentFile?.canonicalFile
            ?: throw IOException("The application resource directory is unavailable")
        val runtime = File(parent, runtimeParent.name)
        requireOwned(runtime)
        if (!runtime.isDirectory && !runtime.mkdirs()) throw IOException("Application resources could not be prepared")
        val web = File(runtime, "web")
        requireOwned(web)
        if (matches(web, index)) {
            removeStaleDirectories(runtime)
            return web
        }

        val staging = File(runtime, ".staging-${UUID.randomUUID()}")
        if (!staging.mkdir()) throw IOException("Application resource staging is unavailable")
        try {
            for (directory in index.directories) {
                val destination = File(staging, directory)
                requireOwned(destination)
                if (!destination.isDirectory && !destination.mkdirs()) throw IOException("A resource directory could not be created")
            }
            for (entry in index.entries) {
                val destination = File(staging, entry.path)
                requireOwned(destination)
                val digest = MessageDigest.getInstance("SHA-256")
                var written = 0L
                openAsset("web/${entry.path}").use { input ->
                    FileOutputStream(destination).use { output ->
                        val buffer = ByteArray(32 * 1024)
                        while (true) {
                            val count = input.read(buffer)
                            if (count < 0) break
                            written += count
                            if (written > entry.bytes) throw IOException("A packaged web resource exceeds its declared length")
                            digest.update(buffer, 0, count)
                            output.write(buffer, 0, count)
                        }
                    }
                }
                if (written != entry.bytes || hex(digest.digest()) != entry.sha256) {
                    throw IOException("A packaged web resource failed its checksum")
                }
            }

            val previous = File(runtime, ".previous-${UUID.randomUUID()}")
            val hadPrevious = web.exists()
            if (hadPrevious && !web.renameTo(previous)) throw IOException("Previous application resources could not be retained")
            if (!staging.renameTo(web)) {
                if (hadPrevious) previous.renameTo(web)
                throw IOException("Prepared application resources could not be published")
            }
            removeStaleDirectories(runtime)
            return web
        } finally {
            removeOwnedTree(staging)
        }
    }

    private fun readIndex(input: InputStream): Index {
        val bytes = ByteArrayOutputStream()
        input.use {
            val buffer = ByteArray(32 * 1024)
            while (true) {
                val count = it.read(buffer)
                if (count < 0) break
                require(bytes.size() + count <= MAX_INDEX_BYTES) { "The web index exceeds its size limit" }
                bytes.write(buffer, 0, count)
            }
        }
        val raw = bytes.toByteArray()
        require(raw.all { (it.toInt() and 0xff) < 128 }) { "The web index must use portable resource names" }
        val text = raw.toString(Charsets.US_ASCII)
        val separator = text.indexOf('\n')
        require(separator >= 0 && text.endsWith('\n')) { "The web index is incomplete" }
        val header = text.substring(0, separator).split('\t')
        require(header.size == 4 && header[0] == "INOUTWEB1" && checksum.matches(header[1])) { "The web index format is invalid" }
        val count = header[2].toIntOrNull()
        val total = header[3].toLongOrNull()
        require(count != null && count in 1..MAX_FILES && total != null && total in 0..MAX_BYTES) { "The web index exceeds resource limits" }
        val table = text.substring(separator + 1)
        require(hex(MessageDigest.getInstance("SHA-256").digest(table.toByteArray(Charsets.US_ASCII))) == header[1]) { "The web index checksum is invalid" }
        val rows = table.dropLast(1).split('\n')
        require(rows.size == count) { "The web index file count differs" }
        val entries = rows.map { row ->
            val fields = row.split('\t')
            require(fields.size == 3 && checksum.matches(fields[0])) { "A web index entry is invalid" }
            val length = fields[1].toLongOrNull()
            val parts = fields[2].split('/')
            require(length != null && length in 0..MAX_BYTES
                && resourcePath.matches(fields[2]) && parts.size <= MAX_DEPTH
                && parts.none { it.isEmpty() || it == "." || it == ".." }) { "A web resource path or length is invalid" }
            Entry(fields[2], length, fields[0])
        }
        require(entries.sumOf { it.bytes } == total && entries.sumOf { it.bytes } <= MAX_BYTES) { "The web index byte count differs" }
        require(entries.zipWithNext().all { (left, right) -> left.path < right.path }) { "Web resource paths must be unique and sorted" }
        require(entries.any { it.path == "index.html" }) { "The web entrypoint is missing" }
        val directories = mutableSetOf("_next")
        for (entry in entries) {
            val parts = entry.path.split('/')
            for (depth in 1 until parts.size) directories.add(parts.take(depth).joinToString("/"))
        }
        require(entries.none { it.path in directories }) { "A resource file conflicts with a directory" }
        return Index(entries, directories)
    }

    private fun matches(web: File, index: Index): Boolean {
        if (!web.isDirectory) return false
        val byName = index.entries.associateBy { it.path }
        val remaining = byName.keys.toMutableSet()
        val pending = ArrayDeque<File>()
        pending.add(web)
        var scanned = 0
        try {
            while (pending.isNotEmpty()) {
                val directory = pending.removeLast()
                requireOwned(directory)
                val children = directory.listFiles() ?: return false
                for (child in children) {
                    if (++scanned > MAX_FILES * MAX_DEPTH) return false
                    requireOwned(child)
                    val name = child.relativeTo(web).invariantSeparatorsPath
                    if (child.isDirectory) {
                        if (name !in index.directories) return false
                        pending.add(child)
                    } else {
                        val entry = byName[name] ?: return false
                        if (!child.isFile || child.length() != entry.bytes) return false
                        val digest = MessageDigest.getInstance("SHA-256")
                        var read = 0L
                        FileInputStream(child).use { input ->
                            val buffer = ByteArray(32 * 1024)
                            while (true) {
                                val count = input.read(buffer)
                                if (count < 0) break
                                read += count
                                if (read > entry.bytes) return false
                                digest.update(buffer, 0, count)
                            }
                        }
                        if (read != entry.bytes || hex(digest.digest()) != entry.sha256) return false
                        remaining.remove(name)
                    }
                }
            }
        } catch (_: IOException) {
            return false
        }
        return remaining.isEmpty() && File(web, "_next").isDirectory
    }

    private fun requireOwned(file: File) {
        if (file.canonicalFile != file.absoluteFile) throw IOException("Symbolic resource destinations are not allowed")
    }

    private fun removeStaleDirectories(runtime: File) {
        runtime.listFiles()?.filter { it.name.startsWith(".staging-") || it.name.startsWith(".previous-") }
            ?.forEach { removeOwnedTree(it) }
    }

    private fun removeOwnedTree(file: File, depth: Int = 0) {
        // Never follow an alias while removing an obsolete owned snapshot.
        if (depth > MAX_DEPTH) return
        if (file.canonicalFile != file.absoluteFile) {
            file.delete()
            return
        }
        if (file.isDirectory) file.listFiles()?.forEach { removeOwnedTree(it, depth + 1) }
        file.delete()
    }

    private fun hex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it) }
}
