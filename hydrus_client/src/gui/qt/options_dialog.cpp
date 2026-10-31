#include "options_dialog.hpp"

// WS-15.1.3c1: options dialog moved from bridge.cpp (move-only split; inline
// methods outlined, no logic change).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QTabWidget>
#include <QVBoxLayout>
#include <QVector>

HydrusOptionsDialog::HydrusOptionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("options"));
    resize(720, 600);

    auto *layout = new QVBoxLayout(this);

    const bool search_bar_top = state.value(QStringLiteral("search_bar_top")).toBool(true);

    auto *tabs = new QTabWidget(this);

    search = new QLineEdit(this);
    search->setPlaceholderText(QStringLiteral("Search options... (Experimental!)"));

    if (search_bar_top) {
        layout->addWidget(search);
        layout->addWidget(tabs, 1);
    } else {
        layout->addWidget(tabs, 1);
        layout->addWidget(search);
    }

    const QJsonArray tab_array = state.value(QStringLiteral("tabs")).toArray();

    for (const QJsonValue &tab_value : tab_array) {
        const QJsonObject tab = tab_value.toObject();
        auto *page = new QWidget(tabs);
        auto *form = new QFormLayout(page);
        form->setContentsMargins(8, 8, 8, 8);

        for (const QJsonValue &field_value : tab.value(QStringLiteral("fields")).toArray()) {
            const QJsonObject field = field_value.toObject();
            OptionField entry;
            entry.key = field.value(QStringLiteral("key")).toString();
            entry.type = field.value(QStringLiteral("type")).toString();
            entry.noneable = field.value(QStringLiteral("noneable")).toBool(false);
            entry.min_value = field.contains(QStringLiteral("min"))
                ? field.value(QStringLiteral("min")).toInt()
                : -2'000'000'000;
            entry.max_value = field.contains(QStringLiteral("max"))
                ? field.value(QStringLiteral("max")).toInt()
                : 2'000'000'000;
            entry.none_phrase = field.value(QStringLiteral("none_phrase")).toString();
            entry.page = page;

            const QString label = field.value(QStringLiteral("label")).toString();
            entry.label = label;
            auto *label_widget = new QLabel(label, page);
            entry.label_widget = label_widget;
            const QString help = field.value(QStringLiteral("help")).toString();
            if (!help.isEmpty()) {
                label_widget->setToolTip(help);
            }
            const QJsonValue value = field.value(QStringLiteral("value"));

            if (entry.type == QStringLiteral("bool")) {
                entry.boolean = new QCheckBox(page);
                entry.boolean->setChecked(value.toBool());
                form->addRow(label_widget, entry.boolean);
                entry.input_widget = entry.boolean;
            } else if (entry.type == QStringLiteral("int")) {
                entry.integer = new QSpinBox(page);
                entry.integer->setRange(entry.min_value, entry.max_value);
                entry.integer->setValue(value.toInt());
                form->addRow(label_widget, entry.integer);
                entry.input_widget = entry.integer;
            } else if (entry.type == QStringLiteral("noneable_int")) {
                entry.noneable_container = new QWidget(page);
                auto *container_layout = new QHBoxLayout(entry.noneable_container);
                container_layout->setContentsMargins(0, 0, 0, 0);
                container_layout->setSpacing(6);
                entry.noneable_integer = new QSpinBox(entry.noneable_container);
                entry.noneable_integer->setRange(entry.min_value, entry.max_value);
                entry.noneable_checkbox = new QCheckBox(
                    entry.none_phrase.isEmpty() ? QStringLiteral("none") : entry.none_phrase,
                    entry.noneable_container);
                if (value.isNull()) {
                    entry.noneable_checkbox->setChecked(true);
                    entry.noneable_integer->setEnabled(false);
                    entry.noneable_integer->setValue(entry.min_value);
                } else {
                    entry.noneable_checkbox->setChecked(false);
                    entry.noneable_integer->setEnabled(true);
                    entry.noneable_integer->setValue(value.toInt());
                }
                QObject::connect(entry.noneable_checkbox, &QCheckBox::toggled, entry.noneable_integer, &QSpinBox::setDisabled);
                container_layout->addWidget(entry.noneable_integer);
                container_layout->addWidget(entry.noneable_checkbox);
                form->addRow(label_widget, entry.noneable_container);
                entry.input_widget = entry.noneable_container;
            } else if (entry.type == QStringLiteral("string")) {
                entry.text = new QLineEdit(page);
                entry.text->setText(value.toString());
                form->addRow(label_widget, entry.text);
                entry.input_widget = entry.text;
            } else {
                auto *readonly = new QLabel(value.toString(), page);
                readonly->setWordWrap(true);
                readonly->setTextInteractionFlags(Qt::TextSelectableByMouse);
                form->addRow(label_widget, readonly);
                entry.input_widget = readonly;
            }

            if (!help.isEmpty() && entry.input_widget != nullptr) {
                entry.input_widget->setToolTip(help);
            }
            fields.append(entry);
        }

        tabs->addTab(page, tab.value(QStringLiteral("title")).toString());
    }

    connect(search, &QLineEdit::textChanged, this, [this, tabs](const QString &text) {
        const QString needle = text.trimmed();

        for (const OptionField &field : fields) {
            const bool visible = needle.isEmpty()
                || field.label.contains(needle, Qt::CaseInsensitive)
                || field.key.contains(needle, Qt::CaseInsensitive);

            if (field.label_widget != nullptr) {
                field.label_widget->setVisible(visible);
            }
            if (field.input_widget != nullptr) {
                field.input_widget->setVisible(visible);
            }
        }

        for (int index = 0; index < tabs->count(); ++index) {
            QWidget *page = tabs->widget(index);
            bool any_visible = false;

            for (const OptionField &field : fields) {
                if (field.page == page
                    && field.input_widget != nullptr
                    && !field.input_widget->isHidden()) {
                    any_visible = true;
                    break;
                }
            }

            tabs->setTabVisible(index, any_visible || needle.isEmpty());
        }
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);
    if (buttons->button(QDialogButtonBox::Save) != nullptr) {
        buttons->button(QDialogButtonBox::Save)->setObjectName(QStringLiteral("HydrusAccept"));
        buttons->button(QDialogButtonBox::Save)->setText(QStringLiteral("apply"));
    }
    if (buttons->button(QDialogButtonBox::Cancel) != nullptr) {
        buttons->button(QDialogButtonBox::Cancel)->setObjectName(QStringLiteral("HydrusCancel"));
        buttons->button(QDialogButtonBox::Cancel)->setText(QStringLiteral("cancel"));
    }
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { apply(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

void HydrusOptionsDialog::apply() {
    QJsonObject booleans;
    QJsonObject integers;
    QJsonObject strings;
    QJsonObject noneable_strings;
    QJsonObject noneable_integers;

    for (const OptionField &field : fields) {
        if (field.boolean != nullptr) {
            booleans.insert(field.key, field.boolean->isChecked());
        } else if (field.integer != nullptr) {
            integers.insert(field.key, field.integer->value());
        } else if (field.noneable_integer != nullptr) {
            if (field.noneable_checkbox != nullptr && field.noneable_checkbox->isChecked()) {
                noneable_integers.insert(field.key, QJsonValue::Null);
            } else {
                noneable_integers.insert(field.key, field.noneable_integer->value());
            }
        } else if (field.text != nullptr) {
            if (field.noneable) {
                noneable_strings.insert(field.key, field.text->text());
            } else {
                strings.insert(field.key, field.text->text());
            }
        }
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("booleans"), booleans);
    payload.insert(QStringLiteral("integers"), integers);
    payload.insert(QStringLiteral("strings"), strings);
    payload.insert(QStringLiteral("noneable_strings"), noneable_strings);
    payload.insert(QStringLiteral("noneable_integers"), noneable_integers);

    main->notify(QStringLiteral("options_apply"), QString(), json_payload(payload));
    accept();
}

void HydrusMainWindow::show_options(const QJsonObject &state) {
    HydrusOptionsDialog dialog(this, state, this);
    dialog.exec();
}
