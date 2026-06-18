// LANGTAIL T1 fixture — representative Kotlin source (Ktor-flavoured).
package com.example.app

import kotlinx.coroutines.runBlocking
import io.ktor.server.application.Application

class Greeter(private val prefix: String) {
    fun greet(name: String): String {
        return "$prefix, $name"
    }

    private fun audit(message: String) {
        println(message)
    }
}

interface Service {
    suspend fun handle()
}

enum class Mood {
    HAPPY,
    GRUMPY
}

object Registry {
    val items = mutableListOf<String>()
}

fun main() = runBlocking {
    println(Greeter("hi").greet("world"))
}
