#include "export_folder.hpp"

// WS-15.1.3b: sidecar exporting section of the edit export folder dialog, layout layer split from export_folder_editor.cpp; logic and texts unchanged.

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QGridLayout>
#include <QJsonObject>
#include <QLabel>
#include <QMenu>
#include <QObject>
#include <QPushButton>
#include <QVBoxLayout>
#include <functional>

void build_export_sidecar_section(
    HydrusMainWindow *main,
    QDialog *dialog,
    QWidget *content,
    QVBoxLayout *vbox,
    const QJsonObject &folder,
    QLineEdit *query,
    std::function<QGridLayout *()> make_grid,
    QString &sidecar_format,
    QCheckBox *&overwrite_next,
    QCheckBox *&overwrite_always) {
    // sidecar exporting
    QVBoxLayout *sidecar_box_layout = nullptr;
    auto *sidecar_box =
        make_static_box(QStringLiteral("sidecar exporting"), content, &sidecar_box_layout);
    auto *sidecar_help = new QLabel(
        QStringLiteral(
            "By default, an export folder will not update pre-existing sidecar files. If you "
            "change the sidecar actions here, or if the metadata has changed and you want those "
            "updates, hit the \"overwrite all sidecars on next run\" checkbox.\n\nYou can force "
            "the export folder to regenerate all your sidecars on every run, but this is an "
            "expensive operation. It is only appropriate for an export folder that runs manually "
            "or rarely.\n\nDO NOT SET YOUR EXPORT FOLDER TO REGEN ALL YOUR SIDECARS EVERY THIRTY "
            "MINUTES."),
        sidecar_box);
    sidecar_help->setWordWrap(true);
    sidecar_box_layout->addWidget(sidecar_help);

    overwrite_next = new QCheckBox(sidecar_box);
    overwrite_next->setChecked(folder.value(QStringLiteral("overwrite_sidecars")).toBool(false));
    overwrite_always = new QCheckBox(sidecar_box);
    overwrite_always->setChecked(
        folder.value(QStringLiteral("always_overwrite_sidecars")).toBool(false));

    auto *sidecar_grid = make_grid();
    auto *overwrite_next_label =
        new QLabel(QStringLiteral("overwrite all sidecars on next run: "), sidecar_box);
    auto *overwrite_always_label = new QLabel(
        QStringLiteral("DANGEROUS: always overwrite all sidecars on every run: "), sidecar_box);
    sidecar_grid->addWidget(overwrite_next_label, 0, 0);
    sidecar_grid->addWidget(overwrite_next, 0, 1, Qt::AlignLeft | Qt::AlignVCenter);
    sidecar_grid->addWidget(overwrite_always_label, 1, 0);
    sidecar_grid->addWidget(overwrite_always, 1, 1, Qt::AlignLeft | Qt::AlignVCenter);
    sidecar_grid->setColumnStretch(1, 1);
    sidecar_box_layout->addLayout(sidecar_grid);

    sidecar_format = folder.value(QStringLiteral("sidecar_format")).toString();

    if (sidecar_format != QStringLiteral("json") && sidecar_format != QStringLiteral("text")) {
        sidecar_format = QStringLiteral("none");
    }

    auto sidecar_label_for = [](const QString &format) {
        if (format == QStringLiteral("json")) {
            return QStringLiteral("1 sidecar: tags.json");
        }

        if (format == QStringLiteral("text")) {
            return QStringLiteral("1 sidecar: tags.txt");
        }

        return QStringLiteral("no sidecars");
    };

    auto *sidecar_button = new QPushButton(sidecar_label_for(sidecar_format), sidecar_box);
    sidecar_box_layout->addWidget(sidecar_button);

    QObject::connect(sidecar_button, &QPushButton::clicked, dialog,
            [sidecar_button, &sidecar_format, sidecar_label_for]() {
                QMenu menu(sidecar_button);
                const struct {
                    const char *label;
                    const char *value;
                } entries[] = {{"no sidecars", "none"},
                               {"tags sidecar (json)", "json"},
                               {"tags sidecar (text)", "text"}};

                for (const auto &entry : entries) {
                    const QString value = QString::fromLatin1(entry.value);
                    auto *action = menu.addAction(QString::fromLatin1(entry.label));
                    action->setCheckable(true);
                    action->setChecked(value == sidecar_format);
                    QObject::connect(action, &QAction::triggered, sidecar_button, [&sidecar_format,
                                                                         sidecar_button,
                                                                         sidecar_label_for,
                                                                         value]() {
                        sidecar_format = value;
                        sidecar_button->setText(sidecar_label_for(sidecar_format));
                    });
                }

                menu.exec(sidecar_button->mapToGlobal(QPoint(0, sidecar_button->height())));
            });

    auto *test_examples = new QPushButton(QStringLiteral("update test example files"), sidecar_box);
    sidecar_box_layout->addWidget(test_examples, 0, Qt::AlignRight);

    QObject::connect(test_examples, &QPushButton::clicked, dialog, [main, query]() {
        QJsonObject payload;
        payload.insert(QStringLiteral("query"), query->text().trimmed());
        main->notify(QStringLiteral("export_folder_test_examples"), QString(), json_payload(payload));
    });
    vbox->addWidget(sidecar_box);
}
