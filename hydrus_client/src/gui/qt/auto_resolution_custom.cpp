#include "auto_resolution.hpp"

// WS-15.1.3a: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTableWidget>
#include <QTabWidget>
#include <QTextEdit>
#include <QTimer>
#include <QVBoxLayout>
#include <algorithm>

HydrusDuplicateCustomActionDialog::HydrusDuplicateCustomActionDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("custom duplicate action"));
    setObjectName(QStringLiteral("hydrus_duplicate_custom_action_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(800, 700);

    data = state;

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose one action and customise the merge and delete options for this pair only. "
                       "Saved merge options are not changed."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    const QJsonObject pair = data.value(QStringLiteral("pair")).toObject();

    if (!pair.isEmpty()) {
        const QJsonObject a = pair.value(QStringLiteral("a")).toObject();
        const QJsonObject b = pair.value(QStringLiteral("b")).toObject();

        auto *summary = new QLabel(
            QStringLiteral("A: %1  %2\nB: %3  %4")
                .arg(a.value(QStringLiteral("hash")).toString().left(16),
                     a.value(QStringLiteral("info")).toString(),
                     b.value(QStringLiteral("hash")).toString().left(16),
                     b.value(QStringLiteral("info")).toString()),
            this);
        summary->setWordWrap(true);
        summary->setStyleSheet(QStringLiteral("font-family: monospace;"));
        layout->addWidget(summary);
    }

    auto *form = new QFormLayout();

    action = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("duplicate_types")).toArray()) {
        const QJsonObject object = entry.toObject();
        action->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    better_side = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("better_sides")).toArray()) {
        const QJsonObject object = entry.toObject();
        better_side->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    delete_mode = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("delete_modes")).toArray()) {
        const QJsonObject object = entry.toObject();
        delete_mode->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    form->addRow(QStringLiteral("action:"), action);
    form->addRow(QStringLiteral("better file:"), better_side);
    form->addRow(QStringLiteral("delete:"), delete_mode);
    layout->addLayout(form);

    tabs = new QTabWidget(this);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("this is better"), better);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("same quality"), same);
    fill_duplicate_merge_page(better, data, data.value(QStringLiteral("better")).toObject());
    fill_duplicate_merge_page(same, data, data.value(QStringLiteral("same")).toObject());
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *apply_button = new QPushButton(QStringLiteral("apply"), this);
    apply_button->setDefault(true);
    buttons->addStretch(1);
    buttons->addWidget(cancel);
    buttons->addWidget(apply_button);
    layout->addLayout(buttons);

    auto update_controls = [this]() {
        const QString kind = action->currentData().toString();
        const bool better_action = kind == QStringLiteral("better");

        better_side->setEnabled(better_action);
        tabs->setTabEnabled(0, better_action);
        tabs->setTabEnabled(1, kind == QStringLiteral("same"));

        if (better_action) {
            tabs->setCurrentIndex(0);
        } else if (kind == QStringLiteral("same")) {
            tabs->setCurrentIndex(1);
        }
    };

    connect(action, &QComboBox::currentIndexChanged, this, [update_controls](int) { update_controls(); });
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(apply_button, &QPushButton::clicked, this, [this]() { apply(); });

    update_controls();
}

void HydrusDuplicateCustomActionDialog::apply() {
    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action->currentData().toString());
    payload.insert(QStringLiteral("better_side"), better_side->currentData().toString());
    payload.insert(QStringLiteral("delete"), delete_mode->currentData().toString());
    payload.insert(QStringLiteral("better"), read_duplicate_merge_page(better));
    payload.insert(QStringLiteral("same"), read_duplicate_merge_page(same));

    main->notify(
        QStringLiteral("duplicates_custom"),
        data.value(QStringLiteral("page_id")).toString(),
        json_payload(payload));

    close();
}
