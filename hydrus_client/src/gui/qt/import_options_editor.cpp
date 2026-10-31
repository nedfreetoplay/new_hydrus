#include "import_options.hpp"

// WS-15.1.3b: import options tab editor moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QTabWidget>
#include <QTableWidget>
#include <QVBoxLayout>
#include <QWidget>

void HydrusImportOptionsDialog::build_editor() {
    editor.tabs = new QTabWidget(this);
    editor.tabs->setObjectName(QStringLiteral("hydrus_import_options_tabs"));

    auto add_page = [this](const QString &key, const QString &title) {
        auto *page = new QWidget(editor.tabs);
        page->setProperty("option_type", key);
        auto *layout = new QVBoxLayout(page);

        auto *use_default = new QCheckBox(
            QStringLiteral("use default import options (inherited from the broader container)"), page);
        layout->addWidget(use_default);

        auto *content = new QWidget(page);
        layout->addWidget(content, 1);

        connect(use_default, &QCheckBox::toggled, content, [content](bool checked) {
            content->setEnabled(!checked);
        });

        editor.use_default.insert(key, use_default);
        editor.content.insert(key, content);
        editor.tabs->addTab(page, title);

        return content;
    };

    auto add_combo_items = [this](QComboBox *combo, const QString &array_key, const QString &label_key, const QString &value_key) {
        for (const QJsonValue &value : data.value(array_key).toArray()) {
            const QJsonObject entry = value.toObject();
            QVariant data_value = entry.value(value_key).toVariant();

            if (data_value.typeId() == QMetaType::Double) {
                data_value = data_value.toInt();
            }

            combo->addItem(entry.value(label_key).toString(), data_value);
        }
    };

    {
        QWidget *content = add_page(QStringLiteral("prefetch"), QStringLiteral("prefetch"));
        auto *form = new QFormLayout(content);

        editor.prefetch_hash = new QComboBox(content);
        editor.prefetch_url = new QComboBox(content);
        editor.prefetch_hash->setObjectName(QStringLiteral("import_options_prefetch_hash"));
        editor.prefetch_url->setObjectName(QStringLiteral("import_options_prefetch_url"));
        add_combo_items(editor.prefetch_hash, QStringLiteral("prefetch_check_types"), QStringLiteral("label"), QStringLiteral("value"));
        add_combo_items(editor.prefetch_url, QStringLiteral("prefetch_check_types"), QStringLiteral("label"), QStringLiteral("value"));

        editor.prefetch_spam = new QCheckBox(QStringLiteral("URL check looks for neighbour spam"), content);
        editor.prefetch_meta_hash = new QCheckBox(QStringLiteral("fetch metadata even if the hash is known"), content);
        editor.prefetch_meta_url = new QCheckBox(QStringLiteral("fetch metadata even if the URL is known"), content);

        form->addRow(QStringLiteral("pre-import hash check:"), editor.prefetch_hash);
        form->addRow(QStringLiteral("pre-import URL check:"), editor.prefetch_url);
        form->addRow(editor.prefetch_spam);
        form->addRow(editor.prefetch_meta_hash);
        form->addRow(editor.prefetch_meta_url);
    }

    {
        QWidget *content = add_page(QStringLiteral("file_filtering"), QStringLiteral("file filtering"));
        auto *layout = new QVBoxLayout(content);
        auto *form = new QFormLayout();

        auto make_size = [content]() {
            auto *spin = new QSpinBox(content);
            spin->setRange(0, 2000000000);
            spin->setSpecialValueText(QStringLiteral("no limit"));
            return spin;
        };
        auto make_resolution = [content]() {
            auto *spin = new QSpinBox(content);
            spin->setRange(0, 100000);
            spin->setSpecialValueText(QStringLiteral("any"));
            return spin;
        };

        editor.min_size = make_size();
        editor.max_size = make_size();
        editor.min_width = make_resolution();
        editor.min_height = make_resolution();
        editor.max_width = make_resolution();
        editor.max_height = make_resolution();
        editor.max_gif_size = make_size();

        auto *min_size_row = new QWidget(content);
        auto *min_size_layout = new QHBoxLayout(min_size_row);
        min_size_layout->setContentsMargins(0, 0, 0, 0);
        min_size_layout->addWidget(editor.min_size);
        min_size_layout->addWidget(new QLabel(QStringLiteral("bytes minimum"), content));

        auto *max_size_row = new QWidget(content);
        auto *max_size_layout = new QHBoxLayout(max_size_row);
        max_size_layout->setContentsMargins(0, 0, 0, 0);
        max_size_layout->addWidget(editor.max_size);
        max_size_layout->addWidget(new QLabel(QStringLiteral("bytes maximum"), content));

        auto *min_res_row = new QWidget(content);
        auto *min_res_layout = new QHBoxLayout(min_res_row);
        min_res_layout->setContentsMargins(0, 0, 0, 0);
        min_res_layout->addWidget(editor.min_width);
        min_res_layout->addWidget(new QLabel(QStringLiteral("x"), content));
        min_res_layout->addWidget(editor.min_height);

        auto *max_res_row = new QWidget(content);
        auto *max_res_layout = new QHBoxLayout(max_res_row);
        max_res_layout->setContentsMargins(0, 0, 0, 0);
        max_res_layout->addWidget(editor.max_width);
        max_res_layout->addWidget(new QLabel(QStringLiteral("x"), content));
        max_res_layout->addWidget(editor.max_height);

        auto *gif_row = new QWidget(content);
        auto *gif_layout = new QHBoxLayout(gif_row);
        gif_layout->setContentsMargins(0, 0, 0, 0);
        gif_layout->addWidget(editor.max_gif_size);
        gif_layout->addWidget(new QLabel(QStringLiteral("bytes maximum for gifs"), content));

        form->addRow(QStringLiteral("minimum size:"), min_size_row);
        form->addRow(QStringLiteral("maximum size:"), max_size_row);
        form->addRow(QStringLiteral("minimum resolution:"), min_res_row);
        form->addRow(QStringLiteral("maximum resolution:"), max_res_row);
        form->addRow(QStringLiteral("maximum animated gif size:"), gif_row);
        layout->addLayout(form);

        editor.exclude_deleted = new QCheckBox(QStringLiteral("exclude previously deleted files"), content);
        editor.allow_bombs = new QCheckBox(QStringLiteral("allow decompression bombs"), content);
        layout->addWidget(editor.exclude_deleted);
        layout->addWidget(editor.allow_bombs);

        layout->addWidget(new QLabel(QStringLiteral("allowed filetypes (none checked means all):"), content));

        editor.mimes = new QListWidget(content);
        editor.mimes->setObjectName(QStringLiteral("import_options_mimes"));
        editor.mimes->setViewMode(QListView::IconMode);
        editor.mimes->setResizeMode(QListView::Adjust);
        editor.mimes->setMovement(QListView::Static);
        editor.mimes->setSpacing(4);

        for (const QJsonValue &value : data.value(QStringLiteral("file_types")).toArray()) {
            const QJsonObject entry = value.toObject();
            auto *item = new QListWidgetItem(
                QStringLiteral("%1  (%2)")
                    .arg(entry.value(QStringLiteral("label")).toString(),
                         entry.value(QStringLiteral("extensions")).toString()),
                editor.mimes);
            item->setData(Qt::UserRole, entry.value(QStringLiteral("value")).toString());
            item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
            item->setCheckState(Qt::Unchecked);
        }

        layout->addWidget(editor.mimes, 1);
    }

    {
        QWidget *content = add_page(QStringLiteral("tag_filtering"), QStringLiteral("tag filtering"));
        auto *layout = new QVBoxLayout(content);

        layout->addWidget(new QLabel(
            QStringLiteral("Blacklist and whitelist slices. A blacklisted parsed tag, or no "
                           "whitelisted tag, makes the import ignored."),
            content));

        editor.tag_rules = new QTableWidget(content);
        editor.tag_rules->setColumnCount(2);
        editor.tag_rules->setHorizontalHeaderLabels({QStringLiteral("tag slice"), QStringLiteral("rule")});
        editor.tag_rules->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
        editor.tag_rules->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
        editor.tag_rules->verticalHeader()->setVisible(false);
        layout->addWidget(editor.tag_rules, 1);

        auto *buttons = new QHBoxLayout();
        auto *add_rule = new QPushButton(QStringLiteral("add rule"), content);
        auto *remove_rule = new QPushButton(QStringLiteral("remove rule"), content);
        buttons->addWidget(add_rule);
        buttons->addWidget(remove_rule);
        buttons->addStretch(1);
        layout->addLayout(buttons);

        connect(add_rule, &QPushButton::clicked, this, [this]() {
            const int row = editor.tag_rules->rowCount();
            editor.tag_rules->insertRow(row);
            editor.tag_rules->setItem(row, 0, new QTableWidgetItem(QStringLiteral("")));
            auto *combo = new QComboBox(editor.tag_rules);
            for (const QJsonValue &value : data.value(QStringLiteral("rules")).toArray()) {
                const QJsonObject entry = value.toObject();
                combo->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
            }
            editor.tag_rules->setCellWidget(row, 1, combo);
        });
        connect(remove_rule, &QPushButton::clicked, this, [this]() {
            const int row = editor.tag_rules->currentRow();

            if (row >= 0) {
                editor.tag_rules->removeRow(row);
            }
        });

        layout->addWidget(new QLabel(QStringLiteral("whitelist tags, one per line:"), content));
        editor.tag_whitelist = new QPlainTextEdit(content);
        layout->addWidget(editor.tag_whitelist, 1);
    }

    {
        QWidget *content = add_page(QStringLiteral("locations"), QStringLiteral("locations"));
        auto *layout = new QVBoxLayout(content);

        layout->addWidget(new QLabel(
            QStringLiteral("Where the file is placed. No destination checked means the default "
                           "local file service."),
            content));

        editor.destinations = new QListWidget(content);
        editor.destinations->setObjectName(QStringLiteral("import_options_destinations"));

        for (const QJsonValue &value : data.value(QStringLiteral("file_services")).toArray()) {
            auto *item = new QListWidgetItem(value.toString(), editor.destinations);
            item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
            item->setCheckState(Qt::Unchecked);
        }

        layout->addWidget(editor.destinations, 1);

        editor.auto_archive = new QCheckBox(QStringLiteral("automatically archive new files"), content);
        editor.assoc_primary = new QCheckBox(QStringLiteral("associate primary URLs"), content);
        editor.assoc_source = new QCheckBox(QStringLiteral("associate source URLs"), content);
        editor.archive_existing = new QCheckBox(QStringLiteral("archive files that are already in db"), content);
        editor.destinations_existing = new QCheckBox(QStringLiteral("add destinations for files already in db"), content);

        for (QCheckBox *checkbox : {editor.auto_archive, editor.assoc_primary, editor.assoc_source,
                                    editor.archive_existing, editor.destinations_existing}) {
            layout->addWidget(checkbox);
        }
    }

    {
        QWidget *content = add_page(QStringLiteral("tags"), QStringLiteral("tags"));
        auto *layout = new QVBoxLayout(content);

        editor.tag_services = new QTableWidget(content);
        editor.tag_services->setColumnCount(7);
        editor.tag_services->setHorizontalHeaderLabels({
            QStringLiteral("tag service"),
            QStringLiteral("get tags"),
            QStringLiteral("additional tags"),
            QStringLiteral("new"),
            QStringLiteral("inbox"),
            QStringLiteral("archive"),
            QStringLiteral("only existing"),
        });
        editor.tag_services->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
        editor.tag_services->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
        editor.tag_services->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
        for (int column = 3; column < 7; ++column) {
            editor.tag_services->horizontalHeader()->setSectionResizeMode(column, QHeaderView::ResizeToContents);
        }
        editor.tag_services->verticalHeader()->setVisible(false);

        const QJsonArray services = data.value(QStringLiteral("tag_services")).toArray();
        editor.tag_services->setRowCount(services.size());

        for (int row = 0; row < services.size(); ++row) {
            const QJsonObject service = services.at(row).toObject();
            auto *name = new QTableWidgetItem(service.value(QStringLiteral("name")).toString());
            name->setData(Qt::UserRole, service.value(QStringLiteral("key")).toString());
            editor.tag_services->setItem(row, 0, name);

            auto *get_tags = new QCheckBox(editor.tag_services);
            get_tags->setObjectName(QStringLiteral("import_options_tag_get"));
            editor.tag_services->setCellWidget(row, 1, get_tags);

            auto *additional = new QLineEdit(editor.tag_services);
            additional->setPlaceholderText(QStringLiteral("comma separated"));
            editor.tag_services->setCellWidget(row, 2, additional);

            auto *to_new = new QCheckBox(editor.tag_services);
            auto *to_inbox = new QCheckBox(editor.tag_services);
            auto *to_archive = new QCheckBox(editor.tag_services);
            auto *only_existing = new QCheckBox(editor.tag_services);
            to_new->setObjectName(QStringLiteral("import_options_tag_new"));
            to_inbox->setObjectName(QStringLiteral("import_options_tag_inbox"));
            to_archive->setObjectName(QStringLiteral("import_options_tag_archive"));
            only_existing->setObjectName(QStringLiteral("import_options_tag_only"));
            editor.tag_services->setCellWidget(row, 3, to_new);
            editor.tag_services->setCellWidget(row, 4, to_inbox);
            editor.tag_services->setCellWidget(row, 5, to_archive);
            editor.tag_services->setCellWidget(row, 6, only_existing);
        }

        layout->addWidget(editor.tag_services, 1);
    }

    {
        QWidget *content = add_page(QStringLiteral("notes"), QStringLiteral("notes"));
        auto *layout = new QVBoxLayout(content);

        editor.get_notes = new QCheckBox(QStringLiteral("add notes"), content);
        editor.extend_notes = new QCheckBox(QStringLiteral("extend an existing note if possible"), content);
        layout->addWidget(editor.get_notes);
        layout->addWidget(editor.extend_notes);

        auto *form = new QFormLayout();
        editor.note_conflict = new QComboBox(content);
        editor.note_conflict->setObjectName(QStringLiteral("import_options_note_conflict"));
        add_combo_items(editor.note_conflict, QStringLiteral("note_conflicts"), QStringLiteral("label"), QStringLiteral("value"));
        form->addRow(QStringLiteral("conflict resolution:"), editor.note_conflict);

        editor.note_all_override = new QLineEdit(content);
        editor.note_all_override->setPlaceholderText(QStringLiteral("optional new name for all notes"));
        form->addRow(QStringLiteral("rename all notes to:"), editor.note_all_override);
        layout->addLayout(form);

        layout->addWidget(new QLabel(QStringLiteral("only these note names, one per line (empty means all):"), content));
        editor.note_whitelist = new QPlainTextEdit(content);
        layout->addWidget(editor.note_whitelist, 1);

        layout->addWidget(new QLabel(QStringLiteral("renames, one per line as name=new name:"), content));
        editor.note_overrides = new QPlainTextEdit(content);
        layout->addWidget(editor.note_overrides, 1);
    }

    {
        QWidget *content = add_page(QStringLiteral("presentation"), QStringLiteral("presentation"));
        auto *form = new QFormLayout(content);

        editor.presentation_status = new QComboBox(content);
        editor.presentation_inbox = new QComboBox(content);
        editor.presentation_status->setObjectName(QStringLiteral("import_options_presentation_status"));
        editor.presentation_inbox->setObjectName(QStringLiteral("import_options_presentation_inbox"));
        add_combo_items(editor.presentation_status, QStringLiteral("presentation_statuses"), QStringLiteral("label"), QStringLiteral("value"));
        add_combo_items(editor.presentation_inbox, QStringLiteral("presentation_inboxes"), QStringLiteral("label"), QStringLiteral("value"));

        form->addRow(QStringLiteral("present:"), editor.presentation_status);
        form->addRow(QStringLiteral("inbox filter:"), editor.presentation_inbox);
    }

    {
        QWidget *content = add_page(QStringLiteral("external_programs"), QStringLiteral("external programs"));
        auto *layout = new QVBoxLayout(content);

        layout->addWidget(new QLabel(
            QStringLiteral("Run a program for each imported file. The file path is appended to the "
                           "arguments when the arguments do not contain {path}."),
            content));

        editor.external_programs = new QTableWidget(content);
        editor.external_programs->setObjectName(QStringLiteral("import_options_external_programs"));
        editor.external_programs->setColumnCount(5);
        editor.external_programs->setHorizontalHeaderLabels({
            QStringLiteral("name"),
            QStringLiteral("program"),
            QStringLiteral("arguments"),
            QStringLiteral("new files"),
            QStringLiteral("already in db"),
        });
        editor.external_programs->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
        editor.external_programs->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
        editor.external_programs->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
        editor.external_programs->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
        editor.external_programs->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
        editor.external_programs->verticalHeader()->setVisible(false);
        layout->addWidget(editor.external_programs, 1);

        auto *buttons = new QHBoxLayout();
        auto *add_program = new QPushButton(QStringLiteral("add program"), content);
        auto *remove_program = new QPushButton(QStringLiteral("remove program"), content);
        buttons->addWidget(add_program);
        buttons->addWidget(remove_program);
        buttons->addStretch(1);
        layout->addLayout(buttons);

        connect(add_program, &QPushButton::clicked, this, [this]() {
            const int row = editor.external_programs->rowCount();
            editor.external_programs->insertRow(row);

            for (int column = 0; column < 3; ++column) {
                auto *edit = new QLineEdit(editor.external_programs);
                editor.external_programs->setCellWidget(row, column, edit);
            }

            editor.external_programs->setCellWidget(row, 3, new QCheckBox(editor.external_programs));
            editor.external_programs->setCellWidget(row, 4, new QCheckBox(editor.external_programs));
        });
        connect(remove_program, &QPushButton::clicked, this, [this]() {
            const int row = editor.external_programs->currentRow();

            if (row >= 0) {
                editor.external_programs->removeRow(row);
            }
        });
    }
}
