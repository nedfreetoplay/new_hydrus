// WS-15.2.1: typed seam implementations (Rust structs, no JSON).
//
// This translation unit is the home for HydrusMainWindow methods whose
// parameters are cxx shared structs owned by api.rs. It includes ONLY the
// generated header <hydrus_client/src/gui/qt/api.cxx.h> (plus Qt and the
// declaration-only support/strings.hpp): the generated header pulls the
// exported header set, so including local topic headers here would define
// every widget class twice. As more seams convert in WS-15.2.2, their typed
// method bodies land here (split per screen if it grows past 500 lines).

#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/strings.hpp"

#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHeaderView>
#include <QLabel>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTabWidget>
#include <QTableWidget>
#include <QUrl>
#include <QVBoxLayout>

// The class definition (members, other methods) comes from the exported
// bridge.hpp chain included above; the method declarations live in
// main_window.hpp next to forward declarations of the structs.

void HydrusMainWindow::set_status(const StatusBar &status) {
    const QString media = to_qstring(status.media);
    const QString message = to_qstring(status.message);

    status_media->setText(media.isEmpty() ? message : media);
    status_media->setToolTip(message.isEmpty() ? media : message);
    status_bandwidth->setText(to_qstring(status.bandwidth));
    status_idle->setText(to_qstring(status.idle));
    status_hydrus_busy->setText(to_qstring(status.hydrus_busy));
    status_cpu_busy->setText(to_qstring(status.cpu_busy));
    status_db->setText(to_qstring(status.db));
}

void HydrusMainWindow::show_about(const AboutInfo &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("about"));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *title = new QLabel(QStringLiteral("Hydrus Network"), &dialog);
    title->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold;"));
    layout->addWidget(title);

    auto *subtitle = new QLabel(
        QStringLiteral("Rust/Qt client · software v%1 · network v%2")
            .arg(state.software_version)
            .arg(state.network_version),
        &dialog);
    layout->addWidget(subtitle);

    auto *tabs = new QTabWidget(&dialog);
    layout->addWidget(tabs, 1);

    auto *overview = new QWidget(tabs);
    auto *form = new QFormLayout(overview);
    const auto add_row = [form](const QString &label, const QString &value) {
        auto *field = new QLabel(value, form->parentWidget());
        field->setTextInteractionFlags(Qt::TextSelectableByMouse);
        field->setWordWrap(true);
        form->addRow(label, field);
    };

    add_row(QStringLiteral("software version"), QString::number(state.software_version));
    add_row(QStringLiteral("network version"), QString::number(state.network_version));
    add_row(QStringLiteral("client api version"), QString::number(state.client_api_version));
    add_row(QStringLiteral("db version"), QString::number(state.db_version));
    add_row(QStringLiteral("client api"), to_qstring(state.api));
    add_row(QStringLiteral("sqlite"), to_qstring(state.sqlite));
    add_row(QStringLiteral("rust crate"), to_qstring(state.rust_crate));
    add_row(QStringLiteral("Qt"), QString::fromLatin1(qVersion()));
    add_row(QStringLiteral("services"), QString::number(state.services));
    add_row(QStringLiteral("file locations"), QString::number(state.locations));
    add_row(QStringLiteral("shortcuts"), QString::number(state.shortcuts));
    add_row(QStringLiteral("db dir"), to_qstring(state.db_dir));
    add_row(QStringLiteral("install dir"), to_qstring(state.install_dir));
    tabs->addTab(overview, QStringLiteral("overview"));

    auto *license = new QPlainTextEdit(tabs);
    license->setReadOnly(true);
    license->setPlainText(
        to_qstring(state.license) + QStringLiteral("\n\n") + to_qstring(state.site));
    tabs->addTab(license, QStringLiteral("license"));

    auto *credits = new QPlainTextEdit(tabs);
    credits->setReadOnly(true);
    credits->setPlainText(to_qstring(state.credits));
    tabs->addTab(credits, QStringLiteral("credits"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *site = new QPushButton(QStringLiteral("open site"), &dialog);
    buttons->addButton(site, QDialogButtonBox::ActionRole);
    const QString site_url = to_qstring(state.site);
    connect(site, &QPushButton::clicked, &dialog, [site_url]() {
        QDesktopServices::openUrl(QUrl(site_url));
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_viewing_stats(const ViewingStats &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("file viewing statistics"));
    dialog.resize(560, 260);

    auto *layout = new QVBoxLayout(&dialog);
    auto *hash = new QLabel(
        QStringLiteral("file: %1").arg(to_qstring(state.hash)),
        &dialog);
    hash->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(hash);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("canvas"),
        QStringLiteral("views"),
        QStringLiteral("viewtime (s)"),
        QStringLiteral("last viewed"),
    });
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);

    table->setRowCount(static_cast<int>(state.rows.size()));

    for (size_t row = 0; row < state.rows.size(); ++row) {
        const ViewingStatRow &entry = state.rows[row];
        const int table_row = static_cast<int>(row);
        table->setItem(table_row, 0, new QTableWidgetItem(to_qstring(entry.canvas)));
        table->setItem(table_row, 1, new QTableWidgetItem(QString::number(entry.views)));
        // Parity: the JSON seam rendered this integer through toDouble, so keep
        // the double formatting for large values instead of switching to int.
        table->setItem(
            table_row, 2,
            new QTableWidgetItem(QString::number(static_cast<double>(entry.viewtime_secs))));
        table->setItem(table_row, 3, new QTableWidgetItem(to_qstring(entry.last_viewed)));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}
