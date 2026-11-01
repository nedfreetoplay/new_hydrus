
// WS-15.1.3b: edit import folder dialog moved from bridge.cpp; the filename tagging section lives in import_folder_tagging.cpp (layout layer split, logic unchanged).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFileDialog>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QStringList>
#include <QVBoxLayout>

void HydrusMainWindow::show_import_folder(const ImportFolderEditorState &state) {
    const QString original = to_qstring(state.original);

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("edit import folder"));
    dialog.setObjectName(QStringLiteral("edit_import_folder_dialog"));
    dialog.resize(700, 760);

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

    auto pretty_delta = [](qint64 seconds) {
        if (seconds <= 0) {
            return QStringLiteral("0 seconds");
        }

        const struct {
            const char *name;
            qint64 size;
        } units[] = {{"day", 86'400}, {"hour", 3'600}, {"minute", 60}, {"second", 1}};

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

    // folder options
    QVBoxLayout *folder_box_layout = nullptr;
    auto *folder_box =
        make_static_box(QStringLiteral("folder options"), content, &folder_box_layout);

    auto *name = new QLineEdit(to_qstring(state.name), folder_box);

    auto *path = new QLineEdit(to_qstring(state.path), folder_box);
    auto *browse = new QPushButton(QStringLiteral("browse"), folder_box);
    auto *path_row = new QWidget(folder_box);
    auto *path_row_layout = new QHBoxLayout(path_row);
    path_row_layout->setContentsMargins(0, 0, 0, 0);
    path_row_layout->setSpacing(2);
    path_row_layout->addWidget(path, 1);
    path_row_layout->addWidget(browse, 0);

    auto *search_subdirectories = new QCheckBox(folder_box);
    search_subdirectories->setChecked(state.search_subdirectories);

    auto *paused = new QCheckBox(folder_box);
    paused->setChecked(state.paused);

    auto *check_regularly = new QCheckBox(folder_box);
    check_regularly->setChecked(state.check_regularly);

    int period_value = int(state.period);
    auto *period_button = new QPushButton(to_qstring(state.period_label), folder_box);
    period_button->setMinimumHeight(24);

    int skip_value = int(state.skip);
    auto *skip_button = new QPushButton(to_qstring(state.skip_label), folder_box);
    skip_button->setMinimumHeight(24);
    skip_button->setToolTip(QStringLiteral("If a file has a modified time more recent than this long ago, it will not be imported in the current check. Helps to avoid importing files that are in the process of downloading/copying (usually on a NAS where other \"already in use\" checks may fail)."));

    auto *check_now = new QCheckBox(folder_box);
    check_now->setChecked(state.check_now);

    auto *show_working_popup = new QCheckBox(folder_box);
    show_working_popup->setChecked(state.show_working_popup);

    auto *publish_popup = new QCheckBox(folder_box);
    publish_popup->setChecked(state.publish_popup);

    auto *publish_page = new QCheckBox(folder_box);
    publish_page->setChecked(state.publish_page);

    auto *folder_grid = make_grid();
    int row = 0;
    auto add_row = [folder_grid, folder_box](int &r, const QString &label, QWidget *widget) {
        folder_grid->addWidget(new QLabel(label, folder_box), r, 0);
        folder_grid->addWidget(widget, r, 1);
        ++r;
    };
    add_row(row, QStringLiteral("name: "), name);
    add_row(row, QStringLiteral("folder path: "), path_row);
    add_row(row, QStringLiteral("search subdirectories: "), search_subdirectories);
    add_row(row, QStringLiteral("currently paused (if set, will not ever do any work): "), paused);
    add_row(row, QStringLiteral("check regularly?: "), check_regularly);
    add_row(row, QStringLiteral("check period: "), period_button);
    add_row(row, QStringLiteral("recent modified time skip period: "), skip_button);
    add_row(row, QStringLiteral("check on manage dialog ok: "), check_now);
    add_row(row, QStringLiteral("show a popup while working: "), show_working_popup);
    add_row(row, QStringLiteral("publish presented files to a popup button: "), publish_popup);
    add_row(row, QStringLiteral("publish presented files to a page: "), publish_page);
    folder_grid->setColumnStretch(1, 1);
    folder_box_layout->addLayout(folder_grid);
    vbox->addWidget(folder_box);

    // source file actions
    QVBoxLayout *file_box_layout = nullptr;
    auto *file_box =
        make_static_box(QStringLiteral("source file actions"), content, &file_box_layout);

    auto *action_grid = make_grid();
    action_grid->setColumnStretch(1, 1);

    auto make_choice = [file_box]() {
        auto *choice = new QComboBox(file_box);
        choice->addItem(QStringLiteral("delete the source file"), QStringLiteral("delete"));
        choice->addItem(QStringLiteral("leave the source file alone, do not reattempt it"), QStringLiteral("ignore"));
        choice->addItem(QStringLiteral("move the source file"), QStringLiteral("move"));
        return choice;
    };

    const QString action_value = to_qstring(state.action);
    auto set_combo = [&action_value](QComboBox *combo) {
        const int index = combo->findData(action_value.isEmpty() ? QStringLiteral("ignore") : action_value);
        combo->setCurrentIndex(index >= 0 ? index : 0);
    };

    auto *action_successful = make_choice();
    set_combo(action_successful);
    const QString action_path_text = to_qstring(state.action_path);
    auto *location_successful = new QLineEdit(action_path_text, file_box);
    auto *action_row_successful = new QWidget(file_box);
    auto *action_layout_successful = new QHBoxLayout(action_row_successful);
    action_layout_successful->setContentsMargins(0, 0, 0, 0);
    action_layout_successful->setSpacing(2);
    action_layout_successful->addWidget(location_successful, 1);
    auto *browse_successful = new QPushButton(QStringLiteral("browse"), action_row_successful);
    action_layout_successful->addWidget(browse_successful, 0);

    auto *action_redundant = make_choice();
    set_combo(action_redundant);
    auto *location_redundant = new QLineEdit(action_path_text, file_box);
    auto *action_row_redundant = new QWidget(file_box);
    auto *action_layout_redundant = new QHBoxLayout(action_row_redundant);
    action_layout_redundant->setContentsMargins(0, 0, 0, 0);
    action_layout_redundant->setSpacing(2);
    action_layout_redundant->addWidget(location_redundant, 1);
    auto *browse_redundant = new QPushButton(QStringLiteral("browse"), action_row_redundant);
    action_layout_redundant->addWidget(browse_redundant, 0);

    auto *action_deleted = make_choice();
    set_combo(action_deleted);
    auto *location_deleted = new QLineEdit(action_path_text, file_box);
    auto *action_row_deleted = new QWidget(file_box);
    auto *action_layout_deleted = new QHBoxLayout(action_row_deleted);
    action_layout_deleted->setContentsMargins(0, 0, 0, 0);
    action_layout_deleted->setSpacing(2);
    action_layout_deleted->addWidget(location_deleted, 1);
    auto *browse_deleted = new QPushButton(QStringLiteral("browse"), action_row_deleted);
    action_layout_deleted->addWidget(browse_deleted, 0);

    auto *action_failed = make_choice();
    set_combo(action_failed);
    auto *location_failed = new QLineEdit(action_path_text, file_box);
    auto *action_row_failed = new QWidget(file_box);
    auto *action_layout_failed = new QHBoxLayout(action_row_failed);
    action_layout_failed->setContentsMargins(0, 0, 0, 0);
    action_layout_failed->setSpacing(2);
    action_layout_failed->addWidget(location_failed, 1);
    auto *browse_failed = new QPushButton(QStringLiteral("browse"), action_row_failed);
    action_layout_failed->addWidget(browse_failed, 0);

    // Python has separate action_path per status, but Rust stores one action_path. Use one for all for simplicity? 
    // Actually Rust ImportFolderEntry has single action_path. Python has per-status locations. Rust model doesn't support per-status. Keep one shared location field.
    // To reduce mismatch, still show four rows but locations share same value.

    auto add_action_row = [action_grid, file_box](int &r, const QString &label, QComboBox *combo, QWidget *location_row) {
        action_grid->addWidget(new QLabel(label, file_box), r, 0);
        action_grid->addWidget(combo, r, 1);
        action_grid->addWidget(location_row, r, 2);
        ++r;
    };

    int arow = 0;
    add_action_row(arow, QStringLiteral("when a file imports successfully: "), action_successful, action_row_successful);
    add_action_row(arow, QStringLiteral("when a file is already in the db: "), action_redundant, action_row_redundant);
    add_action_row(arow, QStringLiteral("when a file has previously been deleted from the db: "), action_deleted, action_row_deleted);
    add_action_row(arow, QStringLiteral("when a file fails to import: "), action_failed, action_row_failed);

    file_box_layout->addLayout(action_grid);
    vbox->addWidget(file_box);

    // Typed tagging entries from Rust; converted to the local QJson shape the
    // tagging section edits in place (not a seam: no parse, no serialize).
    QJsonObject filename_tagging;
    {
        const QStringList slot_keys = {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                                   QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")};
        for (const TaggingEntry &entry : state.tagging) {
            QJsonObject options;
            QJsonArray all_tags;
            for (const rust::String &tag : entry.tags_for_all) {
                all_tags.append(to_qstring(tag));
            }
            options.insert(QStringLiteral("tags_for_all"), all_tags);
            options.insert(QStringLiteral("add_filename"), entry.add_filename);
            options.insert(QStringLiteral("filename_namespace"), to_qstring(entry.filename_namespace));
            QJsonObject directories;
            for (int i = 0; i < slot_keys.size() && i < int(entry.dir_enabled.size()) && i < int(entry.dir_namespaces.size()); ++i) {
                directories.insert(slot_keys.at(i), QJsonObject{
                    {QStringLiteral("enabled"), entry.dir_enabled[size_t(i)]},
                    {QStringLiteral("namespace"), to_qstring(entry.dir_namespaces[size_t(i)])},
                });
            }
            options.insert(QStringLiteral("directories"), directories);
            QJsonArray quick;
            for (int i = 0; i < int(entry.quick_namespaces.size()) && i < int(entry.quick_regexes.size()); ++i) {
                quick.append(QJsonObject{
                    {QStringLiteral("namespace"), to_qstring(entry.quick_namespaces[size_t(i)])},
                    {QStringLiteral("regex"), to_qstring(entry.quick_regexes[size_t(i)])},
                });
            }
            options.insert(QStringLiteral("quick_namespaces"), quick);
            QJsonArray patterns;
            for (const rust::String &pattern : entry.regexes) {
                patterns.append(to_qstring(pattern));
            }
            options.insert(QStringLiteral("regexes"), patterns);
            filename_tagging.insert(to_qstring(entry.service_key), options);
        }
    }
    QJsonObject tagging_result = filename_tagging;
    const QString folder_path = to_qstring(state.path);
    QStringList service_keys;
    QStringList service_names;
    for (const TagServiceRef &service : state.tag_services) {
        service_keys.append(to_qstring(service.key));
        service_names.append(to_qstring(service.name));
    }

    build_import_folder_tagging_section(
        this, &dialog, content, vbox, service_keys, service_names, folder_path, tagging_result);
    filename_tagging = tagging_result;


    auto *note = new QLabel(
        QStringLiteral("Filters are applied before a file is queued. Move/delete runs only for "
                       "files that imported successfully."),
        content);
    note->setWordWrap(true);
    vbox->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    outer->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    auto sync_action = [action_successful, location_successful, browse_successful,
                        action_redundant, location_redundant, browse_redundant,
                        action_deleted, location_deleted, browse_deleted,
                        action_failed, location_failed, browse_failed](QComboBox *combo) {
        const bool move = combo->currentData().toString() == QStringLiteral("move");
        // We only sync the changed combo's row; but simpler: sync all? We'll sync based on sender? 
    };
    // Instead, connect each combo individually.
    auto make_sync = [](QComboBox *combo, QLineEdit *location, QPushButton *browse) {
        auto sync = [combo, location, browse]() {
            const bool move = combo->currentData().toString() == QStringLiteral("move");
            location->setEnabled(move);
            browse->setEnabled(move);
        };
        QObject::connect(combo, QOverload<int>::of(&QComboBox::currentIndexChanged), combo, sync);
        sync();
    };
    make_sync(action_successful, location_successful, browse_successful);
    make_sync(action_redundant, location_redundant, browse_redundant);
    make_sync(action_deleted, location_deleted, browse_deleted);
    make_sync(action_failed, location_failed, browse_failed);

    // period buttons
    connect(period_button, &QPushButton::clicked, &dialog, [&dialog, period_button, &period_value, pretty_delta]() {
        QDialog editor(&dialog);
        editor.setWindowTitle(QStringLiteral("edit time delta"));
        editor.resize(320, 60);

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

    connect(skip_button, &QPushButton::clicked, &dialog, [&dialog, skip_button, &skip_value, pretty_delta]() {
        QDialog editor(&dialog);
        editor.setWindowTitle(QStringLiteral("edit time delta"));
        editor.resize(400, 60);

        auto *editor_layout = new QVBoxLayout(&editor);
        auto *row = new QHBoxLayout();
        row->setContentsMargins(0, 0, 0, 0);
        row->setSpacing(4);

        const int days = static_cast<int>(skip_value / 86'400);
        const int hours = static_cast<int>(skip_value % 86'400 / 3'600);
        const int minutes = static_cast<int>(skip_value % 3'600 / 60);
        const int seconds = static_cast<int>(skip_value % 60);

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
        auto *seconds_spin = new QSpinBox(&editor);
        seconds_spin->setRange(0, 59);
        seconds_spin->setValue(seconds);
        seconds_spin->setFixedWidth(45);

        row->addWidget(days_spin, 1);
        row->addWidget(new QLabel(QStringLiteral("days"), &editor));
        row->addWidget(hours_spin, 0);
        row->addWidget(new QLabel(QStringLiteral("hours"), &editor));
        row->addWidget(minutes_spin, 0);
        row->addWidget(new QLabel(QStringLiteral("minutes"), &editor));
        row->addWidget(seconds_spin, 0);
        row->addWidget(new QLabel(QStringLiteral("seconds"), &editor));
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
            skip_value = std::max(1, days_spin->value() * 86'400 +
                                    hours_spin->value() * 3'600 +
                                    minutes_spin->value() * 60 +
                                    seconds_spin->value());
            skip_button->setText(pretty_delta(skip_value));
        }
    });

    connect(check_regularly, &QCheckBox::toggled, &dialog, [check_regularly, period_button]() {
        period_button->setEnabled(check_regularly->isChecked());
    });
    period_button->setEnabled(check_regularly->isChecked());

    connect(path, &QLineEdit::textChanged, &dialog, [this, &dialog, path, &folder_path]() {});
    // browse
    connect(browse, &QPushButton::clicked, &dialog, [this, &dialog, path]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose an import folder"), path->text());
        if (!chosen.isEmpty()) {
            path->setText(chosen);
        }
    });
    connect(browse_successful, &QPushButton::clicked, &dialog, [this, &dialog, location_successful]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose the destination folder"), location_successful->text());
        if (!chosen.isEmpty()) {
            location_successful->setText(chosen);
        }
    });
    connect(browse_redundant, &QPushButton::clicked, &dialog, [this, &dialog, location_redundant]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose the destination folder"), location_redundant->text());
        if (!chosen.isEmpty()) {
            location_redundant->setText(chosen);
        }
    });
    connect(browse_deleted, &QPushButton::clicked, &dialog, [this, &dialog, location_deleted]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose the destination folder"), location_deleted->text());
        if (!chosen.isEmpty()) {
            location_deleted->setText(chosen);
        }
    });
    connect(browse_failed, &QPushButton::clicked, &dialog, [this, &dialog, location_failed]() {
        const QString chosen = QFileDialog::getExistingDirectory(&dialog, QStringLiteral("choose the destination folder"), location_failed->text());
        if (!chosen.isEmpty()) {
            location_failed->setText(chosen);
        }
    });

    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, path, action_successful, location_successful,
                                                            action_redundant, location_redundant,
                                                            action_deleted, location_deleted,
                                                            action_failed, location_failed]() {
        if (path->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("You must enter a path to import from!"));
            return;
        }

        if (action_successful->currentData().toString() == QStringLiteral("move")
            && location_successful->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("You must enter a path for your successful file move location!"));
            return;
        }

        if (action_redundant->currentData().toString() == QStringLiteral("move")
            && location_redundant->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("You must enter a path for your redundant file move location!"));
            return;
        }

        if (action_deleted->currentData().toString() == QStringLiteral("move")
            && location_deleted->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("You must enter a path for your deleted file move location!"));
            return;
        }

        if (action_failed->currentData().toString() == QStringLiteral("move")
            && location_failed->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("import folder"), QStringLiteral("You must enter a path for your failed file move location!"));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    // Use the successful action for the folder's single action (Rust stores one action).
    const QByteArray active_bytes = active_page.toUtf8();
    const QByteArray original_bytes = original.toUtf8();
    const QByteArray folder_name_bytes = name->text().trimmed().toUtf8();
    const QByteArray folder_path_bytes = path->text().trimmed().toUtf8();
    const QByteArray action_bytes = action_successful->currentData().toString().toUtf8();
    const QByteArray action_path_bytes = location_successful->text().trimmed().toUtf8();
    app_import_folder_save(
        *this,
        rust::Str(active_bytes.constData(), static_cast<size_t>(active_bytes.size())),
        rust::Str(original_bytes.constData(), static_cast<size_t>(original_bytes.size())),
        rust::Str(folder_name_bytes.constData(), static_cast<size_t>(folder_name_bytes.size())),
        rust::Str(folder_path_bytes.constData(), static_cast<size_t>(folder_path_bytes.size())),
        static_cast<int64_t>(period_value),
        check_regularly->isChecked(),
        search_subdirectories->isChecked(),
        paused->isChecked(),
        check_now->isChecked(),
        static_cast<int64_t>(skip_value),
        show_working_popup->isChecked(),
        publish_popup->isChecked(),
        publish_page->isChecked(),
        rust::Str(action_bytes.constData(), static_cast<size_t>(action_bytes.size())),
        rust::Str(action_path_bytes.constData(), static_cast<size_t>(action_path_bytes.size())),
        filename_tagging_entries(filename_tagging));
}
