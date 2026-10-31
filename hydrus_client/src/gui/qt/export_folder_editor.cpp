#include "export_folder.hpp"

// WS-15.1.3b: edit export folder dialog moved from bridge.cpp; the sidecar exporting section lives in export_folder_sidecar.cpp (layout layer split, logic unchanged).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDialog>
#include <QFileDialog>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QMessageBox>
#include <QPushButton>
#include <QScrollArea>
#include <QVBoxLayout>
#include <QWidget>

void HydrusMainWindow::show_export_folder(const QJsonObject &state) {
    const QJsonObject folder = state.value(QStringLiteral("folder")).toObject();
    const QString original = folder.value(QStringLiteral("name")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("edit export folder"));
    dialog.setObjectName(QStringLiteral("edit_export_folder_dialog"));
    dialog.resize(512, 820);

    auto *outer = new QVBoxLayout(&dialog);
    outer->setContentsMargins(4, 4, 4, 4);
    outer->setSpacing(4);

    auto *scroll = new QScrollArea(&dialog);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto *content = new QWidget(scroll);
    auto *vbox = new QVBoxLayout(content);
    vbox->setContentsMargins(0, 0, 0, 0);
    vbox->setSpacing(4);
    scroll->setWidget(content);
    outer->addWidget(scroll, 1);

    auto make_grid = []() {
        auto *grid = new QGridLayout();
        grid->setContentsMargins(0, 0, 0, 0);
        grid->setHorizontalSpacing(2);
        grid->setVerticalSpacing(2);
        return grid;
    };

    // name and location
    QVBoxLayout *path_box_layout = nullptr;
    auto *path_box =
        make_static_box(QStringLiteral("name and location"), content, &path_box_layout);
    auto *name = new QLineEdit(folder.value(QStringLiteral("name")).toString(), path_box);
    auto *path = new QLineEdit(folder.value(QStringLiteral("path")).toString(), path_box);
    auto *browse = new QPushButton(QStringLiteral("browse"), path_box);
    auto *path_row = new QWidget(path_box);
    auto *path_row_layout = new QHBoxLayout(path_row);
    path_row_layout->setContentsMargins(0, 0, 0, 0);
    path_row_layout->setSpacing(2);
    path_row_layout->addWidget(path, 1);
    path_row_layout->addWidget(browse, 0);

    auto *path_grid = make_grid();
    auto *name_label = new QLabel(QStringLiteral("name: "), path_box);
    auto *folder_path_label = new QLabel(QStringLiteral("folder path: "), path_box);
    path_grid->addWidget(name_label, 0, 0);
    path_grid->addWidget(name, 0, 1);
    path_grid->addWidget(folder_path_label, 1, 0);
    path_grid->addWidget(path_row, 1, 1);
    path_grid->setColumnStretch(1, 1);
    path_box_layout->addLayout(path_grid);
    vbox->addWidget(path_box);

    // type of export
    QVBoxLayout *type_box_layout = nullptr;
    auto *type_box =
        make_static_box(QStringLiteral("type of export"), content, &type_box_layout);
    auto *type_help = new QLabel(
        QStringLiteral(
            "regular - try to export the files to the directory, overwriting if the filesize if "
            "different\n\nsynchronise - try to export the files to the directory, overwriting if "
            "the filesize if different, and delete anything else in the directory\n\nIf you select "
            "synchronise, be careful!"),
        type_box);
    type_help->setWordWrap(true);
    type_box_layout->addWidget(type_help);

    auto *export_type = new QComboBox(type_box);
    export_type->addItem(QStringLiteral("regular"), QStringLiteral("copy"));
    export_type->addItem(QStringLiteral("synchronise"), QStringLiteral("synchronise"));
    {
        const int index = export_type->findData(
            folder.value(QStringLiteral("export_type")).toString() ==
                    QStringLiteral("synchronise")
                ? QStringLiteral("synchronise")
                : QStringLiteral("copy"));
        export_type->setCurrentIndex(index >= 0 ? index : 0);
    }
    type_box_layout->addWidget(export_type);

    auto *delete_from_client = new QCheckBox(type_box);
    delete_from_client->setObjectName(QStringLiteral("HydrusWarning"));
    delete_from_client->setChecked(
        folder.value(QStringLiteral("delete_from_client")).toBool(false));
    auto *export_symlinks = new QCheckBox(type_box);
    export_symlinks->setObjectName(QStringLiteral("HydrusWarning"));
    export_symlinks->setChecked(folder.value(QStringLiteral("export_symlinks")).toBool(false));
    export_symlinks->setToolTip(QStringLiteral(
        "You probably need to run hydrus as Admin for this to work on Windows."));

    auto *type_grid = make_grid();
    auto *delete_label =
        new QLabel(QStringLiteral("trash files in hydrus client after export: "), type_box);
    delete_label->setObjectName(QStringLiteral("HydrusWarning"));
    auto *symlinks_label = new QLabel(QStringLiteral("EXPERIMENTAL: export symlinks"), type_box);
    symlinks_label->setObjectName(QStringLiteral("HydrusWarning"));
    type_grid->addWidget(delete_label, 0, 0);
    type_grid->addWidget(delete_from_client, 0, 1, Qt::AlignLeft | Qt::AlignVCenter);
    type_grid->addWidget(symlinks_label, 1, 0);
    type_grid->addWidget(export_symlinks, 1, 1, Qt::AlignLeft | Qt::AlignVCenter);
    type_grid->setColumnStretch(1, 1);
    type_box_layout->addLayout(type_grid);
    vbox->addWidget(type_box);

    // export period
    QVBoxLayout *period_box_layout = nullptr;
    auto *period_box =
        make_static_box(QStringLiteral("export period"), content, &period_box_layout);

    auto *run_regularly = new QCheckBox(period_box);
    run_regularly->setChecked(folder.value(QStringLiteral("run_regularly")).toBool(true));
    auto *show_working_popup = new QCheckBox(period_box);
    show_working_popup->setChecked(
        folder.value(QStringLiteral("show_working_popup")).toBool(true));
    auto *run_on_ok = new QCheckBox(period_box);
    run_on_ok->setChecked(folder.value(QStringLiteral("run_now")).toBool(false));

    int period_value = folder.value(QStringLiteral("period")).toInt(86'400);

    auto pretty_delta = [](qint64 seconds) {
        if (seconds <= 0) {
            return QStringLiteral("0 seconds");
        }

        const struct {
            const char *name;
            qint64 size;
        } units[] = {{"year", 31'557'600}, {"month", 2'629'800}, {"day", 86'400},
                     {"hour", 3'600},    {"minute", 60},      {"second", 1}};

        QStringList parts;
        qint64 remaining = seconds;

        for (const auto &unit : units) {
            const qint64 quantity = remaining / unit.size;
            remaining %= unit.size;

            if (quantity > 0) {
                parts.append(QStringLiteral("%1 %2%3")
                                 .arg(quantity)
                                 .arg(QString::fromLatin1(unit.name),
                                      quantity > 1 ? QStringLiteral("s") : QString()));

                if (parts.size() == 2) {
                    break;
                }
            } else if (!parts.isEmpty()) {
                break;
            }
        }

        return parts.join(QStringLiteral(" "));
    };

    auto *period_button = new QPushButton(pretty_delta(period_value), period_box);
    period_button->setMinimumHeight(24);

    auto *period_grid = make_grid();
    auto *run_regularly_label = new QLabel(QStringLiteral("run regularly?: "), period_box);
    auto *popup_label =
        new QLabel(QStringLiteral("show popup when working regularly?: "), period_box);
    auto *run_on_ok_label = new QLabel(QStringLiteral("run on dialog ok: "), period_box);
    period_grid->addWidget(run_regularly_label, 0, 0);
    period_grid->addWidget(run_regularly, 0, 1, Qt::AlignLeft | Qt::AlignVCenter);
    period_grid->addWidget(period_button, 1, 0, 1, 2);
    period_grid->addWidget(popup_label, 2, 0);
    period_grid->addWidget(show_working_popup, 2, 1, Qt::AlignLeft | Qt::AlignVCenter);
    period_grid->addWidget(run_on_ok_label, 3, 0);
    period_grid->addWidget(run_on_ok, 3, 1, Qt::AlignLeft | Qt::AlignVCenter);
    period_grid->setColumnStretch(1, 1);
    period_box_layout->addLayout(period_grid);
    vbox->addWidget(period_box);

    connect(period_button, &QPushButton::clicked, &dialog, [&dialog, period_button, &period_value,
                                                            pretty_delta]() {
        QDialog editor(&dialog);
        editor.setWindowTitle(QStringLiteral("edit time delta"));
        editor.resize(360, 60);

        auto *editor_layout = new QVBoxLayout(&editor);
        auto *row = new QHBoxLayout();
        row->setContentsMargins(0, 0, 0, 0);
        row->setSpacing(4);

        const int days = static_cast<int>(period_value / 86'400);
        const int hours = static_cast<int>(period_value % 86'400 / 3'600);
        const int minutes = static_cast<int>(period_value % 3'600 / 60);

        auto *days_spin = new QSpinBox(&editor);
        days_spin->setRange(0, 3'523);
        days_spin->setValue(days);
        days_spin->setFixedWidth(50);
        auto *hours_spin = new QSpinBox(&editor);
        hours_spin->setRange(0, 23);
        hours_spin->setValue(hours);
        hours_spin->setFixedWidth(45);
        auto *minutes_spin = new QSpinBox(&editor);
        minutes_spin->setRange(0, 59);
        minutes_spin->setValue(minutes);
        minutes_spin->setFixedWidth(45);

        row->addWidget(days_spin, 1);
        row->addWidget(new QLabel(QStringLiteral("days"), &editor));
        row->addWidget(hours_spin, 0);
        row->addWidget(new QLabel(QStringLiteral("hours"), &editor));
        row->addWidget(minutes_spin, 0);
        row->addWidget(new QLabel(QStringLiteral("minutes"), &editor));
        editor_layout->addLayout(row);

        auto *buttons = new QHBoxLayout();
        buttons->addStretch(1);
        auto *apply = new QPushButton(QStringLiteral("apply"), &editor);
        apply->setObjectName(QStringLiteral("HydrusAccept"));
        apply->setMinimumSize(75, 24);
        auto *cancel = new QPushButton(QStringLiteral("cancel"), &editor);
        cancel->setObjectName(QStringLiteral("HydrusCancel"));
        cancel->setMinimumSize(75, 24);
        buttons->addWidget(apply);
        buttons->addWidget(cancel);
        editor_layout->addLayout(buttons);

        connect(apply, &QPushButton::clicked, &editor, &QDialog::accept);
        connect(cancel, &QPushButton::clicked, &editor, &QDialog::reject);

        if (editor.exec() == QDialog::Accepted) {
            period_value = std::max(180, days_spin->value() * 86'400 +
                                             hours_spin->value() * 3'600 +
                                             minutes_spin->value() * 60);
            period_button->setText(pretty_delta(period_value));
        }
    });

    connect(run_regularly, &QCheckBox::toggled, &dialog, [run_regularly, period_button,
                                                          show_working_popup]() {
        period_button->setEnabled(run_regularly->isChecked());
        show_working_popup->setEnabled(run_regularly->isChecked());
    });
    period_button->setEnabled(run_regularly->isChecked());
    show_working_popup->setEnabled(run_regularly->isChecked());

    // query to export
    QVBoxLayout *query_box_layout = nullptr;
    auto *query_box = make_static_box(QStringLiteral("query to export"), content, &query_box_layout);

    auto *predicates = new QListWidget(query_box);
    predicates->setUniformItemSizes(true);
    predicates->setFixedHeight(94);
    query_box_layout->addWidget(predicates);

    auto *query_row = new QWidget(query_box);
    auto *query_row_layout = new QHBoxLayout(query_row);
    query_row_layout->setContentsMargins(0, 0, 0, 0);
    query_row_layout->setSpacing(2);
    auto *query = new QLineEdit(query_row);
    query->setText(folder.value(QStringLiteral("query")).toString());
    query->setMinimumHeight(24);
    query->setStyleSheet(QStringLiteral("QLineEdit { background-color: #ebf8ff; }"));
    query_row_layout->addWidget(query, 1);

    auto *query_paste = make_icon_button(
        query_row,
        QStringLiteral("paste"),
        QStringLiteral("You can paste a newline-separated list of regular tags and/or system "
                       "predicates."));
    auto *query_favourite = make_icon_button(
        query_row, QStringLiteral("star"), QStringLiteral("Load or save a favourite search."));
    auto *query_clear = make_icon_button(
        query_row,
        QStringLiteral("clear"),
        QStringLiteral("Clear the search back to an empty page."));
    auto *query_lock = make_icon_button(
        query_row,
        QStringLiteral("lock"),
        QStringLiteral("Lock the current files in view to a fixed system:hash."));
    query_row_layout->addWidget(query_paste, 0);
    query_row_layout->addWidget(query_favourite, 0);
    query_row_layout->addWidget(query_clear, 0);
    query_row_layout->addWidget(query_lock, 0);
    query_box_layout->addWidget(query_row);
    vbox->addWidget(query_box);

    auto refresh_predicates = [predicates, query]() {
        predicates->clear();
        const QStringList lines =
            query->text().split(QLatin1Char('\n'), Qt::SkipEmptyParts);

        if (lines.isEmpty()) {
            predicates->addItem(QStringLiteral("system:everything"));
            predicates->setCurrentRow(0);
            return;
        }

        for (const QString &line : lines) {
            const QString trimmed = line.trimmed();

            if (!trimmed.isEmpty()) {
                predicates->addItem(trimmed);
            }
        }
    };
    refresh_predicates();
    connect(query, &QLineEdit::textChanged, &dialog, [refresh_predicates]() {
        refresh_predicates();
    });
    connect(query_paste, &QToolButton::clicked, &dialog, [query]() {
        const QString text = QApplication::clipboard()->text().trimmed();

        if (!text.isEmpty()) {
            query->setText(text);
        }
    });
    connect(query_favourite, &QToolButton::clicked, &dialog, [this]() {
        notify(QStringLiteral("notify"),
               QString(),
               QStringLiteral("favourite searches are not implemented yet"));
    });
    connect(query_clear, &QToolButton::clicked, &dialog, [query]() {
        query->clear();
    });
    connect(query_lock, &QToolButton::clicked, &dialog, [this]() {
        notify(QStringLiteral("notify"),
               QString(),
               QStringLiteral("search synchronisation is not implemented yet"));
    });

    // filenames
    QVBoxLayout *phrase_box_layout = nullptr;
    auto *phrase_box = make_static_box(QStringLiteral("filenames"), content, &phrase_box_layout);
    auto *phrase_row = new QWidget(phrase_box);
    auto *phrase_row_layout = new QHBoxLayout(phrase_row);
    phrase_row_layout->setContentsMargins(0, 0, 0, 0);
    phrase_row_layout->setSpacing(2);
    auto *phrase = new QLineEdit(folder.value(QStringLiteral("phrase")).toString(), phrase_row);
    phrase->setMinimumHeight(24);
    auto *shortcuts = new QPushButton(QStringLiteral("pattern shortcuts"), phrase_row);
    phrase_row_layout->addWidget(phrase, 1);
    phrase_row_layout->addWidget(shortcuts, 0, Qt::AlignVCenter);
    phrase_box_layout->addWidget(phrase_row);
    vbox->addWidget(phrase_box);

    connect(shortcuts, &QPushButton::clicked, &dialog, [this, shortcuts]() {
        QMenu menu(shortcuts);
        auto *header = menu.addAction(QStringLiteral("click on a phrase to copy to clipboard"));
        header->setEnabled(false);
        QFont header_font = header->font();
        header_font.setBold(true);
        header->setFont(header_font);
        menu.addSeparator();

        const struct {
            const char *label;
            const char *phrase;
        } entries[] = {
            {"unique numerical file id - {file_id}", "{file_id}"},
            {"the file's hash - {hash}", "{hash}"},
            {"all the file's tags - {tags}", "{tags}"},
            {"all the file's non-namespaced tags - {nn tags}", "{nn tags}"},
            {"file order - {#}", "{#}"},
        };

        for (const auto &entry : entries) {
            const QString phrase_text = QString::fromLatin1(entry.phrase);
            menu.addAction(QString::fromLatin1(entry.label), this, [phrase_text]() {
                QApplication::clipboard()->setText(phrase_text);
            });
        }

        menu.addSeparator();
        menu.addAction(QStringLiteral("all instances of a particular namespace - [\u2026]"), this,
                       []() { QApplication::clipboard()->setText(QStringLiteral("[\u2026]")); });
        menu.addSeparator();
        menu.addAction(QStringLiteral("a particular tag, if the file has it - (\u2026)"), this,
                       []() { QApplication::clipboard()->setText(QStringLiteral("(\u2026)")); });

        menu.exec(shortcuts->mapToGlobal(QPoint(0, shortcuts->height())));
    });

    QString sidecar_format;
    QCheckBox *overwrite_next = nullptr;
    QCheckBox *overwrite_always = nullptr;
    build_export_sidecar_section(
        this, &dialog, content, vbox, folder, query, make_grid,
        sidecar_format, overwrite_next, overwrite_always);

    vbox->addStretch(1);

    // type of export switches the trash option off, like Python.
    auto update_type = [export_type, delete_from_client]() {
        const bool synchronise =
            export_type->currentData().toString() == QStringLiteral("synchronise");

        delete_from_client->setEnabled(!synchronise);

        if (synchronise) {
            delete_from_client->setChecked(false);
        }
    };
    connect(export_type, &QComboBox::currentIndexChanged, &dialog, [update_type](int) {
        update_type();
    });
    update_type();

    connect(delete_from_client, &QCheckBox::toggled, &dialog, [&dialog](bool checked) {
        if (checked) {
            QMessageBox::warning(
                &dialog,
                QStringLiteral("warning"),
                QStringLiteral("This will delete the exported files from your client (send them to "
                               "trash) after the export! If you do not know what this means, "
                               "uncheck it!"));
        }
    });

    connect(browse, &QPushButton::clicked, &dialog, [&dialog, path]() {
        const QString chosen = QFileDialog::getExistingDirectory(
            &dialog, QStringLiteral("Select directory"), path->text());

        if (!chosen.isEmpty()) {
            path->setText(QDir::toNativeSeparators(chosen));
        }
    });

    auto *buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), &dialog);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    apply->setMinimumSize(75, 24);
    auto *cancel = new QPushButton(QStringLiteral("cancel"), &dialog);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    cancel->setMinimumSize(75, 24);
    buttons->addWidget(apply);
    buttons->addWidget(cancel);
    outer->addLayout(buttons);

    // Python's DialogThatResizes grows to the panel size, limited by the
    // screen; reproduce that instead of always showing a scrollbar.
    vbox->activate();

    int wanted_height = vbox->sizeHint().height() + 4 + buttons->sizeHint().height() + 12;

    if (dialog.screen() != nullptr) {
        wanted_height = std::min(wanted_height, dialog.screen()->availableGeometry().height() - 40);
    }

    dialog.resize(512, wanted_height);

    connect(cancel, &QPushButton::clicked, &dialog, &QDialog::reject);
    connect(apply, &QPushButton::clicked, &dialog, [&dialog, name, path, phrase,
                                                    delete_from_client]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("warning"),
                                 QStringLiteral("You must enter a name for this export folder!"));
            return;
        }

        if (path->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("warning"),
                                 QStringLiteral("You must enter a folder path to export to!"));
            return;
        }

        const QByteArray phrase_bytes = phrase->text().trimmed().toUtf8();
        const rust::String phrase_error =
            hydrus_validate_export_phrase(rust::Str(phrase_bytes.constData(), phrase_bytes.size()));

        if (phrase_error.size() > 0) {
            QMessageBox::warning(
                &dialog, QStringLiteral("warning"),
                QString::fromUtf8(phrase_error.data(), static_cast<int>(phrase_error.size())));
            return;
        }

        if (delete_from_client->isChecked()) {
            const QMessageBox::StandardButton answer = QMessageBox::question(
                &dialog, QStringLiteral("warning"),
                QStringLiteral("You have set this export folder to delete the files from the "
                               "client (send them to trash) after export! Are you absolutely sure "
                               "this is what you want?"),
                QMessageBox::Yes | QMessageBox::No, QMessageBox::No);

            if (answer != QMessageBox::Yes) {
                return;
            }
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("path"), path->text().trimmed());
    payload.insert(QStringLiteral("query"), query->text().trimmed());
    payload.insert(QStringLiteral("phrase"), phrase->text().trimmed());
    payload.insert(QStringLiteral("export_type"), export_type->currentData().toString());
    payload.insert(QStringLiteral("sidecar_format"), sidecar_format);
    payload.insert(QStringLiteral("overwrite_sidecars"), overwrite_next->isChecked());
    payload.insert(QStringLiteral("always_overwrite_sidecars"), overwrite_always->isChecked());
    payload.insert(QStringLiteral("delete_from_client"), delete_from_client->isChecked());
    payload.insert(QStringLiteral("export_symlinks"), export_symlinks->isChecked());
    payload.insert(QStringLiteral("run_regularly"), run_regularly->isChecked());
    payload.insert(QStringLiteral("show_working_popup"), show_working_popup->isChecked());
    payload.insert(QStringLiteral("run_now"), run_on_ok->isChecked());
    payload.insert(QStringLiteral("period"), period_value);

    notify(QStringLiteral("export_folder_save"), active_page, json_payload(payload));
}
