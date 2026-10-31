#include "import_options.hpp"

// WS-15.1.3b: import options editor read-back moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QPlainTextEdit>
#include <QRegularExpression>
#include <QTableWidget>

void HydrusImportOptionsDialog::fill_editor(const QJsonObject &container, const QJsonObject &fallback) {
    auto set_use_default = [this, &container](const QString &key) {
        const bool use_default = !container.contains(key);
        editor.use_default.value(key)->setChecked(use_default);
        editor.content.value(key)->setEnabled(!use_default);
        return !use_default;
    };

    auto options_for = [&container, &fallback](const QString &key) {
        return (container.contains(key) ? container : fallback).value(key).toObject();
    };

    auto set_combo = [](QComboBox *combo, const QJsonValue &value) {
        QVariant data_value = value.toVariant();

        if (data_value.typeId() == QMetaType::Double) {
            data_value = data_value.toInt();
        }

        const int index = combo->findData(data_value);

        if (index >= 0) {
            combo->setCurrentIndex(index);
        }
    };

    auto array_contains = [](const QJsonArray &array, const QString &needle) {
        for (const QJsonValue &value : array) {
            if (value.toString() == needle) {
                return true;
            }
        }

        return false;
    };

    {
        const bool enabled = set_use_default(QStringLiteral("prefetch"));
        const QJsonObject options = options_for(QStringLiteral("prefetch"));
        set_combo(editor.prefetch_hash, options.value(QStringLiteral("preimport_hash_check_type")));
        set_combo(editor.prefetch_url, options.value(QStringLiteral("preimport_url_check_type")));
        editor.prefetch_spam->setChecked(options.value(QStringLiteral("preimport_url_check_looks_for_neighbour_spam")).toBool(true));
        editor.prefetch_meta_hash->setChecked(options.value(QStringLiteral("fetch_metadata_even_if_hash_known")).toBool());
        editor.prefetch_meta_url->setChecked(options.value(QStringLiteral("fetch_metadata_even_if_url_known")).toBool());
        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("file_filtering"));
        const QJsonObject options = options_for(QStringLiteral("file_filtering"));
        auto number = [&options](const QString &key) {
            return options.value(key).toInt(0);
        };
        editor.min_size->setValue(number(QStringLiteral("min_size")));
        editor.max_size->setValue(number(QStringLiteral("max_size")));
        editor.max_gif_size->setValue(number(QStringLiteral("max_gif_size")));
        editor.min_width->setValue(0);
        editor.min_height->setValue(0);
        editor.max_width->setValue(0);
        editor.max_height->setValue(0);

        auto fill_resolution = [&options](const QString &key, QSpinBox *width, QSpinBox *height) {
            const QJsonArray resolution = options.value(key).toArray();

            if (resolution.size() == 2) {
                width->setValue(resolution.at(0).toInt());
                height->setValue(resolution.at(1).toInt());
            }
        };
        fill_resolution(QStringLiteral("min_resolution"), editor.min_width, editor.min_height);
        fill_resolution(QStringLiteral("max_resolution"), editor.max_width, editor.max_height);

        editor.exclude_deleted->setChecked(options.value(QStringLiteral("excludes_deleted")).toBool(true));
        editor.allow_bombs->setChecked(options.value(QStringLiteral("allows_decompression_bombs")).toBool());

        const QJsonArray allowed = options.value(QStringLiteral("allowed_mimes")).toArray();

        for (int row = 0; row < editor.mimes->count(); ++row) {
            QListWidgetItem *item = editor.mimes->item(row);
            item->setCheckState(array_contains(allowed, item->data(Qt::UserRole).toString()) ? Qt::Checked : Qt::Unchecked);
        }

        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("tag_filtering"));
        const QJsonObject options = options_for(QStringLiteral("tag_filtering"));
        const QJsonObject rules = options.value(QStringLiteral("rules")).toObject();
        editor.tag_rules->setRowCount(0);

        for (auto iterator = rules.constBegin(); iterator != rules.constEnd(); ++iterator) {
            const int row = editor.tag_rules->rowCount();
            editor.tag_rules->insertRow(row);
            editor.tag_rules->setItem(row, 0, new QTableWidgetItem(iterator.key()));
            auto *combo = new QComboBox(editor.tag_rules);

            for (const QJsonValue &value : data.value(QStringLiteral("rules")).toArray()) {
                const QJsonObject entry = value.toObject();
                combo->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
            }

            set_combo(combo, iterator.value());
            editor.tag_rules->setCellWidget(row, 1, combo);
        }

        QStringList whitelist;
        for (const QJsonValue &value : options.value(QStringLiteral("whitelist")).toArray()) {
            whitelist << value.toString();
        }
        editor.tag_whitelist->setPlainText(whitelist.join(QStringLiteral("\n")));
        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("locations"));
        const QJsonObject options = options_for(QStringLiteral("locations"));
        const QJsonArray destinations = options.value(QStringLiteral("destination_services")).toArray();

        for (int row = 0; row < editor.destinations->count(); ++row) {
            QListWidgetItem *item = editor.destinations->item(row);
            item->setCheckState(array_contains(destinations, item->text()) ? Qt::Checked : Qt::Unchecked);
        }

        editor.auto_archive->setChecked(options.value(QStringLiteral("automatically_archives")).toBool());
        editor.assoc_primary->setChecked(options.value(QStringLiteral("associate_primary_urls")).toBool(true));
        editor.assoc_source->setChecked(options.value(QStringLiteral("associate_source_urls")).toBool(true));
        editor.archive_existing->setChecked(options.value(QStringLiteral("do_automatic_archive_on_already_in_db_files")).toBool(true));
        editor.destinations_existing->setChecked(options.value(QStringLiteral("do_import_destinations_on_already_in_db_files")).toBool());
        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("tags"));
        const QJsonObject options = options_for(QStringLiteral("tags"));

        for (int row = 0; row < editor.tag_services->rowCount(); ++row) {
            const QTableWidgetItem *name = editor.tag_services->item(row, 0);
            const QJsonObject service_options = options.value(name->data(Qt::UserRole).toString()).toObject();
            auto *get_tags = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 1));
            auto *additional = qobject_cast<QLineEdit *>(editor.tag_services->cellWidget(row, 2));
            auto *to_new = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 3));
            auto *to_inbox = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 4));
            auto *to_archive = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 5));
            auto *only_existing = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 6));

            get_tags->setChecked(service_options.value(QStringLiteral("get_tags")).toBool());
            QStringList tags;

            for (const QJsonValue &tag : service_options.value(QStringLiteral("additional_tags")).toArray()) {
                tags << tag.toString();
            }

            additional->setText(tags.join(QStringLiteral(", ")));
            to_new->setChecked(service_options.value(QStringLiteral("to_new_files")).toBool(true));
            to_inbox->setChecked(service_options.value(QStringLiteral("to_already_in_inbox")).toBool(true));
            to_archive->setChecked(service_options.value(QStringLiteral("to_already_in_archive")).toBool(true));
            only_existing->setChecked(service_options.value(QStringLiteral("only_add_existing_tags")).toBool());
        }

        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("notes"));
        const QJsonObject options = options_for(QStringLiteral("notes"));
        editor.get_notes->setChecked(options.value(QStringLiteral("get_notes")).toBool(true));
        editor.extend_notes->setChecked(options.value(QStringLiteral("extend_existing_note_if_possible")).toBool(true));
        set_combo(editor.note_conflict, options.value(QStringLiteral("conflict_resolution")).toString());
        editor.note_all_override->setText(options.value(QStringLiteral("all_name_override")).toString());

        QStringList whitelist;
        for (const QJsonValue &value : options.value(QStringLiteral("name_whitelist")).toArray()) {
            whitelist << value.toString();
        }
        editor.note_whitelist->setPlainText(whitelist.join(QStringLiteral("\n")));

        QStringList overrides;
        const QJsonObject names_to_overrides = options.value(QStringLiteral("names_to_name_overrides")).toObject();
        for (auto iterator = names_to_overrides.constBegin(); iterator != names_to_overrides.constEnd(); ++iterator) {
            overrides << QStringLiteral("%1=%2").arg(iterator.key(), iterator.value().toString());
        }
        editor.note_overrides->setPlainText(overrides.join(QStringLiteral("\n")));
        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("presentation"));
        const QJsonObject options = options_for(QStringLiteral("presentation"));
        set_combo(editor.presentation_status, options.value(QStringLiteral("presentation_status")).toString());
        set_combo(editor.presentation_inbox, options.value(QStringLiteral("presentation_inbox")).toString());
        Q_UNUSED(enabled);
    }

    {
        const bool enabled = set_use_default(QStringLiteral("external_programs"));
        const QJsonArray entries = options_for(QStringLiteral("external_programs")).value(QStringLiteral("entries")).toArray();
        editor.external_programs->setRowCount(0);

        for (const QJsonValue &value : entries) {
            const QJsonObject entry = value.toObject();
            const int row = editor.external_programs->rowCount();
            editor.external_programs->insertRow(row);

            auto *name = new QLineEdit(entry.value(QStringLiteral("name")).toString(), editor.external_programs);
            auto *path = new QLineEdit(entry.value(QStringLiteral("path")).toString(), editor.external_programs);
            auto *arguments = new QLineEdit(entry.value(QStringLiteral("arguments")).toString(), editor.external_programs);
            auto *on_new = new QCheckBox(editor.external_programs);
            auto *on_already = new QCheckBox(editor.external_programs);
            on_new->setChecked(entry.value(QStringLiteral("do_it_on_new")).toBool());
            on_already->setChecked(entry.value(QStringLiteral("do_it_on_already_in")).toBool());

            editor.external_programs->setCellWidget(row, 0, name);
            editor.external_programs->setCellWidget(row, 1, path);
            editor.external_programs->setCellWidget(row, 2, arguments);
            editor.external_programs->setCellWidget(row, 3, on_new);
            editor.external_programs->setCellWidget(row, 4, on_already);
        }

        Q_UNUSED(enabled);
    }
}

QJsonObject HydrusImportOptionsDialog::read_editor() const {
    QJsonObject container;

    auto is_used = [this](const QString &key) {
        const QCheckBox *checkbox = editor.use_default.value(key);
        return checkbox != nullptr && !checkbox->isChecked();
    };

    auto combo_value = [](const QComboBox *combo) {
        return combo->currentData().toInt();
    };

    if (is_used(QStringLiteral("prefetch"))) {
        container.insert(QStringLiteral("prefetch"), QJsonObject{
            {QStringLiteral("preimport_hash_check_type"), combo_value(editor.prefetch_hash)},
            {QStringLiteral("preimport_url_check_type"), combo_value(editor.prefetch_url)},
            {QStringLiteral("preimport_url_check_looks_for_neighbour_spam"), editor.prefetch_spam->isChecked()},
            {QStringLiteral("fetch_metadata_even_if_hash_known"), editor.prefetch_meta_hash->isChecked()},
            {QStringLiteral("fetch_metadata_even_if_url_known"), editor.prefetch_meta_url->isChecked()},
        });
    }

    if (is_used(QStringLiteral("file_filtering"))) {
        QJsonArray allowed_mimes;

        for (int row = 0; row < editor.mimes->count(); ++row) {
            const QListWidgetItem *item = editor.mimes->item(row);

            if (item->checkState() == Qt::Checked) {
                allowed_mimes.append(item->data(Qt::UserRole).toString());
            }
        }

        QJsonObject options{
            {QStringLiteral("allowed_mimes"), allowed_mimes},
            {QStringLiteral("excludes_deleted"), editor.exclude_deleted->isChecked()},
            {QStringLiteral("allows_decompression_bombs"), editor.allow_bombs->isChecked()},
        };

        auto set_number = [&options](const QString &key, int value) {
            options.insert(key, value > 0 ? QJsonValue(value) : QJsonValue(QJsonValue::Null));
        };
        set_number(QStringLiteral("min_size"), editor.min_size->value());
        set_number(QStringLiteral("max_size"), editor.max_size->value());
        set_number(QStringLiteral("max_gif_size"), editor.max_gif_size->value());

        auto set_resolution = [&options](const QString &key, int width, int height) {
            if (width > 0 || height > 0) {
                options.insert(key, QJsonArray{width, height});
            } else {
                options.insert(key, QJsonValue(QJsonValue::Null));
            }
        };
        set_resolution(QStringLiteral("min_resolution"), editor.min_width->value(), editor.min_height->value());
        set_resolution(QStringLiteral("max_resolution"), editor.max_width->value(), editor.max_height->value());

        container.insert(QStringLiteral("file_filtering"), options);
    }

    if (is_used(QStringLiteral("tag_filtering"))) {
        QJsonObject rules;

        for (int row = 0; row < editor.tag_rules->rowCount(); ++row) {
            const QTableWidgetItem *slice = editor.tag_rules->item(row, 0);
            auto *combo = qobject_cast<QComboBox *>(editor.tag_rules->cellWidget(row, 1));

            if (slice == nullptr || combo == nullptr || slice->text().trimmed().isEmpty()) {
                continue;
            }

            rules.insert(slice->text().trimmed(), combo->currentData().toString());
        }

        QJsonArray whitelist;
        for (const QString &line : editor.tag_whitelist->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            whitelist.append(line.trimmed());
        }

        container.insert(QStringLiteral("tag_filtering"), QJsonObject{
            {QStringLiteral("rules"), rules},
            {QStringLiteral("whitelist"), whitelist},
        });
    }

    if (is_used(QStringLiteral("locations"))) {
        QJsonArray destinations;

        for (int row = 0; row < editor.destinations->count(); ++row) {
            const QListWidgetItem *item = editor.destinations->item(row);

            if (item->checkState() == Qt::Checked) {
                destinations.append(item->text());
            }
        }

        container.insert(QStringLiteral("locations"), QJsonObject{
            {QStringLiteral("destination_services"), destinations},
            {QStringLiteral("automatically_archives"), editor.auto_archive->isChecked()},
            {QStringLiteral("associate_primary_urls"), editor.assoc_primary->isChecked()},
            {QStringLiteral("associate_source_urls"), editor.assoc_source->isChecked()},
            {QStringLiteral("do_automatic_archive_on_already_in_db_files"), editor.archive_existing->isChecked()},
            {QStringLiteral("do_import_destinations_on_already_in_db_files"), editor.destinations_existing->isChecked()},
        });
    }

    if (is_used(QStringLiteral("tags"))) {
        QJsonObject services;

        for (int row = 0; row < editor.tag_services->rowCount(); ++row) {
            const QTableWidgetItem *name = editor.tag_services->item(row, 0);
            auto *get_tags = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 1));
            auto *additional = qobject_cast<QLineEdit *>(editor.tag_services->cellWidget(row, 2));
            auto *to_new = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 3));
            auto *to_inbox = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 4));
            auto *to_archive = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 5));
            auto *only_existing = qobject_cast<QCheckBox *>(editor.tag_services->cellWidget(row, 6));

            if (name == nullptr) {
                continue;
            }

            QJsonArray tags;
            for (const QString &tag : additional->text().split(QLatin1Char(','), Qt::SkipEmptyParts)) {
                const QString trimmed = tag.trimmed();

                if (!trimmed.isEmpty()) {
                    tags.append(trimmed);
                }
            }

            services.insert(name->data(Qt::UserRole).toString(), QJsonObject{
                {QStringLiteral("get_tags"), get_tags->isChecked()},
                {QStringLiteral("additional_tags"), tags},
                {QStringLiteral("to_new_files"), to_new->isChecked()},
                {QStringLiteral("to_already_in_inbox"), to_inbox->isChecked()},
                {QStringLiteral("to_already_in_archive"), to_archive->isChecked()},
                {QStringLiteral("only_add_existing_tags"), only_existing->isChecked()},
            });
        }

        container.insert(QStringLiteral("tags"), services);
    }

    if (is_used(QStringLiteral("notes"))) {
        QJsonArray whitelist;
        for (const QString &line : editor.note_whitelist->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            whitelist.append(line.trimmed());
        }

        QJsonObject overrides;
        for (const QString &line : editor.note_overrides->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            const int separator = line.indexOf(QLatin1Char('='));

            if (separator > 0) {
                overrides.insert(line.left(separator).trimmed(), line.mid(separator + 1).trimmed());
            }
        }

        const QString all_override = editor.note_all_override->text().trimmed();

        container.insert(QStringLiteral("notes"), QJsonObject{
            {QStringLiteral("get_notes"), editor.get_notes->isChecked()},
            {QStringLiteral("extend_existing_note_if_possible"), editor.extend_notes->isChecked()},
            {QStringLiteral("conflict_resolution"), editor.note_conflict->currentData().toString()},
            {QStringLiteral("name_whitelist"), whitelist},
            {QStringLiteral("all_name_override"), all_override.isEmpty() ? QJsonValue(QJsonValue::Null) : QJsonValue(all_override)},
            {QStringLiteral("names_to_name_overrides"), overrides},
        });
    }

    if (is_used(QStringLiteral("presentation"))) {
        container.insert(QStringLiteral("presentation"), QJsonObject{
            {QStringLiteral("presentation_status"), editor.presentation_status->currentData().toString()},
            {QStringLiteral("presentation_inbox"), editor.presentation_inbox->currentData().toString()},
        });
    }

    if (is_used(QStringLiteral("external_programs"))) {
        QJsonArray entries;

        for (int row = 0; row < editor.external_programs->rowCount(); ++row) {
            auto *name = qobject_cast<QLineEdit *>(editor.external_programs->cellWidget(row, 0));
            auto *path = qobject_cast<QLineEdit *>(editor.external_programs->cellWidget(row, 1));
            auto *arguments = qobject_cast<QLineEdit *>(editor.external_programs->cellWidget(row, 2));
            auto *on_new = qobject_cast<QCheckBox *>(editor.external_programs->cellWidget(row, 3));
            auto *on_already = qobject_cast<QCheckBox *>(editor.external_programs->cellWidget(row, 4));

            if (path == nullptr || path->text().trimmed().isEmpty()) {
                continue;
            }

            entries.append(QJsonObject{
                {QStringLiteral("name"), name->text().trimmed()},
                {QStringLiteral("path"), path->text().trimmed()},
                {QStringLiteral("arguments"), arguments->text()},
                {QStringLiteral("do_it_on_new"), on_new->isChecked()},
                {QStringLiteral("do_it_on_already_in"), on_already->isChecked()},
            });
        }

        container.insert(QStringLiteral("external_programs"), QJsonObject{
            {QStringLiteral("entries"), entries},
        });
    }

    return container;
}
