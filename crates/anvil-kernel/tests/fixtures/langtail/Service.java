// LANGTAIL T1 fixture — representative Java source (Spring-flavoured).
package com.example.app;

import java.util.List;
import java.util.concurrent.CompletableFuture;
import static java.util.Collections.emptyList;

public class Service {
    private final String name;

    public Service(String name) {
        this.name = name;
    }

    public CompletableFuture<List<String>> fetch() {
        return CompletableFuture.completedFuture(emptyList());
    }

    private int helper() {
        return 0;
    }
}

interface Repository {
    List<String> findAll();
}

enum Status {
    ACTIVE,
    INACTIVE
}

record Pair(String left, String right) {}
