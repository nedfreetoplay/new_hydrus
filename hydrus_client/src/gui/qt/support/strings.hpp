#pragma once

#include "rust/cxx.h"

#include <QByteArray>
#include <QJsonArray>
#include <QJsonObject>
#include <QString>
#include <QStringList>

// WS-15.1.1: string/JSON helpers moved verbatim from bridge.cpp.
QString to_qstring(rust::Str text);
QString to_qstring(const rust::String &text);
QString from_rust(rust::Str text);
QJsonObject parse_object(rust::Str text);
QJsonArray parse_array(rust::Str text);
QStringList json_to_strings(const QJsonArray &array);
QByteArray json_bytes(const QJsonObject &object);
QString json_payload(const QJsonObject &object);
