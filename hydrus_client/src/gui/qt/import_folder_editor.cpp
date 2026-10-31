#include "import_folder.hpp"

// WS-15.1.3b: edit import folder dialog moved from bridge.cpp; the filename tagging section lives in import_folder_tagging.cpp (layout layer split, logic unchanged).

#include "bridge.hpp"
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
#include <QVBoxLayout>

void HydrusMainWindow::show_import_folder(const QJsonObject &state) {
    const QJsonObject folder = state.value(QStringLiteral("folder")).toObject();
    const QString original = folder.value(QStringLiteral("name")).toString();

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

    auto *name = new QLineEdit(folder.value(QStringLiteral("name")).toString(), folder_box);

    auto *path = new QLineEdit(folder.value(QStringLiteral("path")).toString(), folder_box);
    auto *browse = new QPushButton(QStringLiteral("browse"), folder_box);
    auto *path_row = new QWidget(folder_box);
    auto *path_row_layout = new QHBoxLayout(path_row);
    path_row_layout->setContentsMargins(0, 0, 0, 0);
    path_row_layout->setSpacing(2);
    path_row_layout->addWidget(path, 1);
    path_row_layout->addWidget(browse, 0);

    auto *search_subdirectories = new QCheckBox(folder_box);
    search_subdirectories->setChecked(folder.value(QStringLiteral("search_subdirectories")).toBool(true));

    auto *paused = new QCheckBox(folder_box);
    paused->setChecked(folder.value(QStringLiteral("paused")).toBool(false));

    auto *check_regularly = new QCheckBox(folder_box);
    check_regularly->setChecked(folder.value(QStringLiteral("check_regularly")).toBool(true));

    int period_value = folder.value(QStringLiteral("period")).toInt(3'600);
    auto *period_button = new QPushButton(pretty_delta(period_value), folder_box);
    period_button->setMinimumHeight(24);

    int skip_value = folder.value(QStringLiteral("last_modified_skip")).toInt(60);
    auto *skip_button = new QPushButton(pretty_delta(skip_value), folder_box);
    skip_button->setMinimumHeight(24);
    skip_button->setToolTip(QStringLiteral("If a file has a modified time more recent than this long ago, it will not be imported in the current check. Helps to avoid importing files that are in the process of downloading/copying (usually on a NAS where other \"already in use\" checks may fail)."));

    auto *check_now = new QCheckBox(folder_box);
    check_now->setChecked(folder.value(QStringLiteral("check_now")).toBool(false));

    auto *show_working_popup = new QCheckBox(folder_box);
    show_working_popup->setChecked(folder.value(QStringLiteral("show_working_popup")).toBool(true));

    auto *publish_popup = new QCheckBox(folder_box);
    publish_popup->setChecked(folder.value(QStringLiteral("publish_files_to_popup_button")).toBool(true));

    auto *publish_page = new QCheckBox(folder_box);
    publish_page->setChecked(folder.value(QStringLiteral("publish_files_to_page")).toBool(false));

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

    const QString action_value = folder.value(QStringLiteral("action")).toString();
    auto set_combo = [&action_value](QComboBox *combo) {
        const int index = combo->findData(action_value.isEmpty() ? QStringLiteral("ignore") : action_value);
        combo->setCurrentIndex(index >= 0 ? index : 0);
    };

    auto *action_successful = make_choice();
    set_combo(action_successful);
    auto *location_successful = new QLineEdit(folder.value(QStringLiteral("action_path")).toString(), file_box);
    auto *action_row_successful = new QWidget(file_box);
    auto *action_layout_successful = new QHBoxLayout(action_row_successful);
    action_layout_successful->setContentsMargins(0, 0, 0, 0);
    action_layout_successful->setSpacing(2);
    action_layout_successful->addWidget(location_successful, 1);
    auto *browse_successful = new QPushButton(QStringLiteral("browse"), action_row_successful);
    action_layout_successful->addWidget(browse_successful, 0);

    auto *action_redundant = make_choice();
    set_combo(action_redundant);
    auto *location_redundant = new QLineEdit(folder.value(QStringLiteral("action_path")).toString(), file_box);
    auto *action_row_redundant = new QWidget(file_box);
    auto *action_layout_redundant = new QHBoxLayout(action_row_redundant);
    action_layout_redundant->setContentsMargins(0, 0, 0, 0);
    action_layout_redundant->setSpacing(2);
    action_layout_redundant->addWidget(location_redundant, 1);
    auto *browse_redundant = new QPushButton(QStringLiteral("browse"), action_row_redundant);
    action_layout_redundant->addWidget(browse_redundant, 0);

    auto *action_deleted = make_choice();
    set_combo(action_deleted);
    auto *location_deleted = new QLineEdit(folder.value(QStringLiteral("action_path")).toString(), file_box);
    auto *action_row_deleted = new QWidget(file_box);
    auto *action_layout_deleted = new QHBoxLayout(action_row_deleted);
    action_layout_deleted->setContentsMargins(0, 0, 0, 0);
    action_layout_deleted->setSpacing(2);
    action_layout_deleted->addWidget(location_deleted, 1);
    auto *browse_deleted = new QPushButton(QStringLiteral("browse"), action_row_deleted);
    action_layout_deleted->addWidget(browse_deleted, 0);

    auto *action_failed = make_choice();
    set_combo(action_failed);
    auto *location_failed = new QLineEdit(folder.value(QStringLiteral("action_path")).toString(), file_box);
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

    QJsonObject filename_tagging = folder.value(QStringLiteral("filename_tagging")).toObject();
    const QString folder_path = folder.value(QStringLiteral("path")).toString();

    build_import_folder_tagging_section(
        this, &dialog, content, vbox, state, folder_path, filename_tagging);


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

    QJsonObject payload;
    payload.insert(QStringLiteral("original"), original);
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("path"), path->text().trimmed());
    payload.insert(QStringLiteral("period"), period_value);
    payload.insert(QStringLiteral("check_regularly"), check_regularly->isChecked());
    payload.insert(QStringLiteral("search_subdirectories"), search_subdirectories->isChecked());
    payload.insert(QStringLiteral("paused"), paused->isChecked());
    payload.insert(QStringLiteral("check_now"), check_now->isChecked());
    payload.insert(QStringLiteral("last_modified_skip"), skip_value);
    payload.insert(QStringLiteral("show_working_popup"), show_working_popup->isChecked());
    payload.insert(QStringLiteral("publish_files_to_popup_button"), publish_popup->isChecked());
    payload.insert(QStringLiteral("publish_files_to_page"), publish_page->isChecked());

    // Use the successful action for the folder's single action (Rust stores one action).
    payload.insert(QStringLiteral("action"), action_successful->currentData().toString());
    payload.insert(QStringLiteral("action_path"), location_successful->text().trimmed());

    payload.insert(QStringLiteral("filename_tagging"), filename_tagging);

    notify(QStringLiteral("import_folder_save"), active_page, json_payload(payload));
}
