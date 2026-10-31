// WS-15.1.3c2: about dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QLabel>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTabWidget>
#include <QUrl>
#include <QVBoxLayout>

void HydrusMainWindow::show_about(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("about"));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *title = new QLabel(QStringLiteral("Hydrus Network"), &dialog);
    title->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold;"));
    layout->addWidget(title);

    auto *subtitle = new QLabel(
        QStringLiteral("Rust/Qt client · software v%1 · network v%2")
            .arg(state.value(QStringLiteral("software_version")).toInt())
            .arg(state.value(QStringLiteral("network_version")).toInt()),
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

    add_row(QStringLiteral("software version"), QString::number(state.value(QStringLiteral("software_version")).toInt()));
    add_row(QStringLiteral("network version"), QString::number(state.value(QStringLiteral("network_version")).toInt()));
    add_row(QStringLiteral("client api version"), QString::number(state.value(QStringLiteral("client_api_version")).toInt()));
    add_row(QStringLiteral("db version"), QString::number(state.value(QStringLiteral("db_version")).toInt()));
    add_row(QStringLiteral("client api"), state.value(QStringLiteral("api")).toString());
    add_row(QStringLiteral("sqlite"), state.value(QStringLiteral("sqlite")).toString());
    add_row(QStringLiteral("rust crate"), state.value(QStringLiteral("rust")).toString());
    add_row(QStringLiteral("Qt"), QString::fromLatin1(qVersion()));
    add_row(QStringLiteral("services"), QString::number(state.value(QStringLiteral("services")).toInt()));
    add_row(QStringLiteral("file locations"), QString::number(state.value(QStringLiteral("locations")).toInt()));
    add_row(QStringLiteral("shortcuts"), QString::number(state.value(QStringLiteral("shortcuts")).toInt()));
    add_row(QStringLiteral("db dir"), state.value(QStringLiteral("db_dir")).toString());
    add_row(QStringLiteral("install dir"), state.value(QStringLiteral("install_dir")).toString());
    tabs->addTab(overview, QStringLiteral("overview"));

    auto *license = new QPlainTextEdit(tabs);
    license->setReadOnly(true);
    license->setPlainText(
        state.value(QStringLiteral("license")).toString() + QStringLiteral("\n\n")
        + state.value(QStringLiteral("site")).toString());
    tabs->addTab(license, QStringLiteral("license"));

    auto *credits = new QPlainTextEdit(tabs);
    credits->setReadOnly(true);
    credits->setPlainText(state.value(QStringLiteral("credits")).toString());
    tabs->addTab(credits, QStringLiteral("credits"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *site = new QPushButton(QStringLiteral("open site"), &dialog);
    buttons->addButton(site, QDialogButtonBox::ActionRole);
    connect(site, &QPushButton::clicked, &dialog, [&state]() {
        QDesktopServices::openUrl(QUrl(state.value(QStringLiteral("site")).toString()));
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}
