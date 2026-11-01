
// WS-15.2.4b: typed edit-timestamps dialog (no QJsonObject).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QCheckBox>
#include <QDateTime>
#include <QDateTimeEdit>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QJsonDocument>
#include <QVBoxLayout>

HydrusTimestampsDialog::HydrusTimestampsDialog(HydrusMainWindow *main_window, const TimestampsDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window), hash(to_qstring(state.hash)) {
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

    set_time(archived_enabled, archived, state.has_archived, static_cast<long long>(state.archived));
    set_time(modified_enabled, modified, state.has_modified, static_cast<long long>(state.modified));

    auto *buttons = new QDialogButtonBox(
        QDialogButtonBox::Save | QDialogButtonBox::Cancel,
        this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

void HydrusTimestampsDialog::set_time(QCheckBox *enabled, QDateTimeEdit *editor, bool has_value, long long secs) {
    if (has_value) {
        enabled->setChecked(true);
        editor->setDateTime(QDateTime::fromSecsSinceEpoch(secs));
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

    main->notify(UiAction::TimestampsSave,
        QString(),
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    accept();
}
