#include "strings.hpp"

#include <QJsonDocument>
#include <QJsonParseError>

QString to_qstring(rust::Str text) {
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

QString from_rust(rust::Str text) {
    return QString::fromUtf8(text.data(), static_cast<int>(text.size()));
}

QJsonObject parse_object(rust::Str text) {
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(QByteArray(text.data(), static_cast<int>(text.size())), &error);

    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return {};
    }

    return document.object();
}

QJsonArray parse_array(rust::Str text) {
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(QByteArray(text.data(), static_cast<int>(text.size())), &error);

    if (error.error != QJsonParseError::NoError || !document.isArray()) {
        return {};
    }

    return document.array();
}

QStringList json_to_strings(const QJsonArray &array) {
    QStringList result;

    for (const QJsonValue &value : array) {
        result.append(value.toString());
    }

    return result;
}

QByteArray json_bytes(const QJsonObject &object) {
    return QJsonDocument(object).toJson(QJsonDocument::Compact);
}

QString json_payload(const QJsonObject &object) {
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}
