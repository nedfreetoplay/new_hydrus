#include "sort_options.hpp"

#include <QColor>
#include <QListWidget>

const HydrusSortOption hydrus_sort_options[] = {
    {"hash id", "sort by hash - sha256", "lexicographic", "reverse lexicographic", true},
    {"import time", "sort by import time", "oldest first", "newest first", false},
    {"size", "sort by file: filesize", "smallest first", "largest first", false},
    {"mime", "sort by filetype", "filetype", "filetype", true},
    {"width", "sort by width", "slimmest first", "widest first", true},
    {"height", "sort by height", "shortest first", "tallest first", true},
    {"ratio", "sort by resolution ratio", "tallest first", "widest first", true},
    {"pixels", "sort by number of pixels", "ascending", "descending", false},
    {"duration", "sort by duration", "shortest first", "longest first", false},
    {"frames", "sort by number of frames", "smallest first", "largest first", false},
    {"words", "sort by number of words", "ascending", "descending", true},
};

const HydrusSortOption *find_sort_option(const QString &key) {
    for (const HydrusSortOption &option : hydrus_sort_options) {
        if (key == QLatin1String(option.key)) {
            return &option;
        }
    }

    return &hydrus_sort_options[2];
}

QStringList default_system_predicates() {
    return {
        QStringLiteral("system:everything"),
        QStringLiteral("system:inbox"),
        QStringLiteral("system:archive"),
        QStringLiteral("system:dimensions"),
        QStringLiteral("system:duration"),
        QStringLiteral("system:file properties"),
        QStringLiteral("system:file relationships"),
        QStringLiteral("system:file service"),
        QStringLiteral("system:file viewing statistics"),
        QStringLiteral("system:filesize"),
        QStringLiteral("system:filetype"),
        QStringLiteral("system:hash"),
        QStringLiteral("system:limit"),
        QStringLiteral("system:notes"),
        QStringLiteral("system:number of tags"),
        QStringLiteral("system:number of words"),
        QStringLiteral("system:rating"),
        QStringLiteral("system:similar files"),
        QStringLiteral("system:tag (advanced)"),
        QStringLiteral("system:tag as number"),
        QStringLiteral("system:time"),
        QStringLiteral("system:urls"),
    };
}

void fill_system_predicates(QListWidget *list) {
    list->clear();

    for (const QString &text : default_system_predicates()) {
        auto *item = new QListWidgetItem(text, list);
        item->setForeground(QColor(150, 90, 10));
    }

    if (list->count() > 0) {
        list->setCurrentRow(0);
    }
}
