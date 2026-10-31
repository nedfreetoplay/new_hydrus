#include "timestamps_dialog.hpp"

// WS-15.1.3c2: edit timestamps dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QDateTime>
#include <QDateTimeEdit>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QJsonDocument>
#include <QVBoxLayout>

HydrusTimestampsDialog::HydrusTimestampsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window), hash(state.value(QStringLiteral("hash")).toString()) {
    setWindowTitle(QStringLiteral("edit timestamps"));
    resize(420, 220);

    auto *layout = new QVBoxLayout(this);
    auto *form = new QFormLayout();

    archived_enabled = new QCheckBox(QStringLiteral("archived time"), this);
    archived = new QDateTimeEdit(this);
    archived->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
    archived->setCalendarPopup(true);
    form->addRow(archived_enabled, archived);

    modified_enabled = new QCheckBox(QStringLiteral("file modified time"), this);
    modified = new QDateTimeEdit(this);
    modified->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
    modified->setCalendarPopup(true);
    form->addRow(modified_enabled, modified);

    layout->addLayout(form);

    set_time(archived_enabled, archived, state.value(QStringLiteral("archived")));
    set_time(modified_enabled, modified, state.value(QStringLiteral("modified")));

    auto *buttons = new QDialogButtonBox(
        QDialogButtonBox::Save | QDialogButtonBox::Cancel,
        this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

void HydrusTimestampsDialog::set_time(QCheckBox *enabled, QDateTimeEdit *editor, const QJsonValue &value) {
    if (value.isDouble()) {
        enabled->setChecked(true);
        editor->setDateTime(QDateTime::fromSecsSinceEpoch(value.toInteger()));
    } else {
        enabled->setChecked(false);
        editor->setDateTime(QDateTime::currentDateTime());
    }
}

void HydrusTimestampsDialog::save() {
    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(
        QStringLiteral("archived"),
        archived_enabled->isChecked()
            ? QJsonValue(archived->dateTime().toSecsSinceEpoch())
            : QJsonValue(QJsonValue::Null));
    payload.insert(
        QStringLiteral("modified"),
        modified_enabled->isChecked()
            ? QJsonValue(modified->dateTime().toSecsSinceEpoch())
            : QJsonValue(QJsonValue::Null));

    main->notify(
        QStringLiteral("timestamps_save"),
        QString(),
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    accept();
}
