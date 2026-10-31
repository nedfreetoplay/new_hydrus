#include "or_predicate_dialog.hpp"

// WS-15.1.3c1: OR predicate editor moved from bridge.cpp (move-only split;
// inline methods outlined, no logic change).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QLabel>
#include <QPlainTextEdit>
#include <QStringList>
#include <QVBoxLayout>

HydrusOrDialog::HydrusOrDialog(const QString &initial, QWidget *parent)
    : QDialog(parent) {
    setWindowTitle(QStringLiteral("OR predicates"));
    resize(520, 380);

    auto *layout = new QVBoxLayout(this);
    auto *help = new QLabel(
        QStringLiteral("One predicate per line. A file matches if any line matches.\n"
                       "Tags, wildcards, namespaces and system: predicates are supported."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    editor = new QPlainTextEdit(this);
    editor->setPlainText(initial);
    editor->setPlaceholderText(QStringLiteral("blue eyes\nspecies:wolf\nsystem:width > 1000"));
    layout->addWidget(editor, 1);

    auto *buttons = new QDialogButtonBox(
        QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
        this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

QString HydrusOrDialog::expression() const {
    QStringList parts;

    for (const QString &line : editor->toPlainText().split(QLatin1Char('\n'))) {
        const QString trimmed = line.trimmed();

        if (!trimmed.isEmpty()) {
            parts.append(trimmed);
        }
    }

    if (parts.isEmpty()) {
        return {};
    }

    return QStringLiteral("{%1}").arg(parts.join(QStringLiteral(" | ")));
}

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial) {
    HydrusOrDialog dialog(initial, &main);

    if (dialog.exec() == QDialog::Accepted) {
        const QString expression = dialog.expression();

        if (!expression.isEmpty()) {
            main.notify(QStringLiteral("or_insert"), page_id, expression);
        }
    }
}
