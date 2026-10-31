// WS-15.1.3c2: tag filter dialog moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialog>
#include <QDialogButtonBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTabWidget>
#include <QVBoxLayout>

void HydrusMainWindow::show_tag_filter(const QJsonObject &state) {
    const QString service_key = state.value(QStringLiteral("service_key")).toString();
    const QString name = state.value(QStringLiteral("name")).toString();
    const bool can_purge = state.value(QStringLiteral("can_purge")).toBool(false);

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("tag filter: ") + name);
    dialog.setObjectName(QStringLiteral("hydrus_tag_filter_dialog"));
    dialog.resize(760, 640);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("current filter: %1").arg(state.value(QStringLiteral("filter_string")).toString()),
        &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    const QString note = state.value(QStringLiteral("note")).toString();

    if (!note.isEmpty()) {
        auto *note_label = new QLabel(note, &dialog);
        note_label->setWordWrap(true);
        layout->addWidget(note_label);
    }

    auto *explanation = new QLabel(
        QStringLiteral("One tag slice per line. Bullet points are namespaces (\"character\" or \"character:\"); "
                       "\"*\" matches all unnamespaced tags and \"*:*\" matches all namespaced tags. "
                       "Whitelist rules are exceptions to blacklist rules."),
        &dialog);
    explanation->setWordWrap(true);
    layout->addWidget(explanation);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setObjectName(QStringLiteral("tag_filter_whitelist"));
    whitelist->setPlaceholderText(QStringLiteral("character\ncreator:someone"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setObjectName(QStringLiteral("tag_filter_blacklist"));
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*\nspecies:chat"));
    tabs->addTab(whitelist, QStringLiteral("whitelist"));
    tabs->addTab(blacklist, QStringLiteral("blacklist"));
    layout->addWidget(tabs, 1);

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &entry : state.value(QStringLiteral("rules")).toArray()) {
        const QJsonObject rule = entry.toObject();
        const QString display = rule.value(QStringLiteral("display")).toString();
        const QString kind = rule.value(QStringLiteral("rule")).toString();

        if (kind == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            QJsonObject entry;
            entry.insert(QStringLiteral("slice"), slice);
            entry.insert(QStringLiteral("rule"), rule);
            rules.append(entry);
        }

        return rules;
    };

    auto build_payload = [&]() {
        QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

        for (const QJsonValue &entry : collect_rules(blacklist, QStringLiteral("blacklist"))) {
            rules.append(entry);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("rules"), rules);

        return payload;
    };

    auto *test_row = new QHBoxLayout();
    auto *test_input = new QLineEdit(&dialog);
    test_input->setPlaceholderText(QStringLiteral("character:samus"));
    auto *test_button = new QPushButton(QStringLiteral("test tag"), &dialog);
    test_row->addWidget(new QLabel(QStringLiteral("test:"), &dialog));
    test_row->addWidget(test_input, 1);
    test_row->addWidget(test_button);
    layout->addLayout(test_row);

    connect(test_button, &QPushButton::clicked, this, [this, test_input, build_payload]() {
        QJsonObject payload = build_payload();
        payload.insert(QStringLiteral("tag"), test_input->text().trimmed());
        notify(QStringLiteral("tag_filter_test"), QString(), json_payload(payload));
    });
    connect(test_input, &QLineEdit::returnPressed, test_button, &QPushButton::click);

    auto *purge_tab = new QWidget(&dialog);
    auto *purge_layout = new QVBoxLayout(purge_tab);
    auto *purge_note = new QLabel(
        can_purge
            ? QStringLiteral("Deletes every mapping of these tags from the local tag service.")
            : QStringLiteral("Purging is only available for local tag services in this build."),
        purge_tab);
    purge_note->setWordWrap(true);
    purge_layout->addWidget(purge_note);

    auto *purge_tags = new QPlainTextEdit(purge_tab);
    purge_tags->setPlaceholderText(QStringLiteral("character:samus\ncreator:someone"));
    purge_tags->setEnabled(can_purge);
    purge_layout->addWidget(purge_tags, 1);

    auto *reason_row = new QHBoxLayout();
    auto *purge_reason = new QLineEdit(purge_tab);
    purge_reason->setPlaceholderText(QStringLiteral("Tags purged by janitor."));
    purge_reason->setEnabled(can_purge);
    reason_row->addWidget(new QLabel(QStringLiteral("reason:"), purge_tab));
    reason_row->addWidget(purge_reason, 1);
    purge_layout->addLayout(reason_row);

    auto *purge_button = new QPushButton(QStringLiteral("purge tags"), purge_tab);
    purge_button->setEnabled(can_purge);
    auto *purge_footer = new QHBoxLayout();
    purge_footer->addStretch(1);
    purge_footer->addWidget(purge_button);
    purge_layout->addLayout(purge_footer);
    tabs->addTab(purge_tab, QStringLiteral("purge"));

    connect(purge_button, &QPushButton::clicked, this, [this, service_key, purge_tags, purge_reason, purge_button]() {
        const QStringList tags = purge_tags->toPlainText().split(QLatin1Char('\n'), Qt::SkipEmptyParts);

        if (tags.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("purge tags"), QStringLiteral("Enter tags first."));
            return;
        }

        if (QMessageBox::question(this, QStringLiteral("purge tags"),
                                  QStringLiteral("Purge %1 tag(s) from this service?").arg(tags.size()))
            != QMessageBox::Yes) {
            return;
        }

        QJsonArray list;

        for (const QString &tag : tags) {
            if (!tag.trimmed().isEmpty()) {
                list.append(tag.trimmed());
            }
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("tags"), list);
        payload.insert(QStringLiteral("reason"), purge_reason->text().trimmed());

        notify(QStringLiteral("purge_tags"), QString(), json_payload(payload));
        purge_tags->clear();
        purge_button->setEnabled(false);
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, this, [&]() {
        notify(QStringLiteral("tag_filter_save"), QString(), json_payload(build_payload()));
        dialog.accept();
    });

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_TAG_FILTER_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    dialog.exec();
}
