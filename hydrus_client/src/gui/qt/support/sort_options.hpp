#pragma once

#include <QString>
#include <QStringList>

class QListWidget;

// WS-15.1.1: sort options and system predicates moved verbatim from bridge.cpp.
struct HydrusSortOption {
    const char *key;
    const char *label;
    const char *ascending;
    const char *descending;
    bool default_ascending;
};

extern const HydrusSortOption hydrus_sort_options[11];

const HydrusSortOption *find_sort_option(const QString &key);
QStringList default_system_predicates();
void fill_system_predicates(QListWidget *list);
