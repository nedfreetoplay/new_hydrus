#pragma once

#include <QIcon>
#include <QString>

// WS-15.1.1: icon helper moved verbatim from bridge.cpp. The `static` on the
// file-name table was dropped so bridge.cpp can keep using it; no logic change.
QString hydrus_small_icon_file_name(const QString &kind);
QIcon hydrus_small_icon(const QString &kind);
