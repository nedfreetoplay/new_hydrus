#include "import_options.hpp"

// WS-15.1.3b: import options dialog shell moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QDialogButtonBox>
#include <QHBoxLayout>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QListWidget>
#include <QListWidgetItem>
#include <QMessageBox>
#include <QPushButton>
#include <QSplitter>
#include <QVBoxLayout>

HydrusImportOptionsDialog::HydrusImportOptionsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("import options"));
    setObjectName(QStringLiteral("hydrus_import_options_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(1120, 760);

    data = state;
    const QJsonObject specific = state.value(QStringLiteral("specific")).toObject();
    specific_mode = !specific.isEmpty();

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        specific_mode
            ? QStringLiteral("These options belong to this one importer. A tab with \"use default "
                             "import options\" checked inherits the caller default and then global "
                             "options; set only what should differ.")
            : QStringLiteral("Each caller type starts from the global options. Set only what should differ; "
                       "a tab with \"use default import options\" checked is inherited from the broader "
                       "container. Favourites are named containers you can reuse elsewhere."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *splitter = new QSplitter(Qt::Horizontal, this);

    auto *left = new QWidget(splitter);
    left_panel = left;
    auto *left_layout = new QVBoxLayout(left);
    left_layout->setContentsMargins(0, 0, 0, 0);
    left_layout->addWidget(new QLabel(QStringLiteral("containers"), left));

    list = new QListWidget(left);
    left_layout->addWidget(list, 1);

    auto *list_buttons = new QHBoxLayout();
    auto *add_button = new QPushButton(QStringLiteral("new favourite"), left);
    rename_button = new QPushButton(QStringLiteral("rename"), left);
    delete_button = new QPushButton(QStringLiteral("delete"), left);
    list_buttons->addWidget(add_button);
    list_buttons->addWidget(rename_button);
    list_buttons->addWidget(delete_button);
    left_layout->addLayout(list_buttons);

    auto *right = new QWidget(splitter);
    auto *right_layout = new QVBoxLayout(right);
    right_layout->setContentsMargins(0, 0, 0, 0);

    description = new QLabel(right);
    description->setWordWrap(true);
    right_layout->addWidget(description);

    build_editor();
    right_layout->addWidget(editor.tabs, 1);

    splitter->addWidget(left);
    splitter->addWidget(right);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({240, 820});

    if (specific_mode) {
        left->setVisible(false);
        splitter->setSizes({0, 1060});
    }

    layout->addWidget(splitter, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);
    layout->addWidget(buttons);

    connect(add_button, &QPushButton::clicked, this, [this]() { add_favourite(); });
    connect(rename_button, &QPushButton::clicked, this, [this]() { rename_favourite(); });
    connect(delete_button, &QPushButton::clicked, this, [this]() { delete_favourite(); });
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    connect(list, &QListWidget::currentRowChanged, this, [this](int) {
        if (rebuilding) {
            return;
        }

        store_current();

        const QString key = current_key();

        if (key.isEmpty()) {
            return;
        }

        selected_key = key;

        QJsonObject defaults = data.value(QStringLiteral("defaults")).toObject();
        QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();
        QString name = key;
        QJsonObject container;

        if (name.startsWith(QStringLiteral("default:"))) {
            name = name.mid(8);
            container = defaults.value(name).toObject();
            const QJsonArray callers = data.value(QStringLiteral("caller_types")).toArray();

            for (const QJsonValue &value : callers) {
                const QJsonObject caller = value.toObject();

                if (caller.value(QStringLiteral("key")).toString() == name) {
                    description->setText(caller.value(QStringLiteral("description")).toString());
                    break;
                }
            }
        } else {
            name = name.mid(10);
            container = favourites.value(name).toObject();
            description->setText(QStringLiteral("favourite container \"%1\"").arg(name));
        }

        rename_button->setEnabled(!selected_key.startsWith(QStringLiteral("default:")));
        delete_button->setEnabled(!selected_key.startsWith(QStringLiteral("default:")));

        fill_editor(container, defaults.value(QStringLiteral("global")).toObject());
    });

    rebuild_list();

    if (specific_mode) {
        description->setText(
            QStringLiteral("options specific to this importer; unchecked tabs inherit the caller default"));

        const QJsonObject container = specific.value(QStringLiteral("container")).toObject();
        const QJsonObject fallback = data.value(QStringLiteral("fallback")).toObject();
        fill_editor(container, fallback);
    } else {

    const QString selected = data.value(QStringLiteral("selected")).toString();

    if (!selected.isEmpty()) {
        for (int row = 0; row < list->count(); ++row) {
            if (list->item(row)->data(Qt::UserRole).toString() == selected) {
                list->setCurrentRow(row);
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_IMPORT_OPTIONS_FAVOURITE")) {
        const QString target = qEnvironmentVariable("HYDRUS_QT_IMPORT_OPTIONS_FAVOURITE");

        for (int row = 0; row < list->count(); ++row) {
            if (list->item(row)->data(Qt::UserRole).toString() == target) {
                list->setCurrentRow(row);
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_IMPORT_OPTIONS_CALLER")) {
        const QString target = QStringLiteral("default:%1").arg(qEnvironmentVariable("HYDRUS_QT_IMPORT_OPTIONS_CALLER"));

        for (int row = 0; row < list->count(); ++row) {
            if (list->item(row)->data(Qt::UserRole).toString() == target) {
                list->setCurrentRow(row);
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_IMPORT_OPTIONS_TAB")) {
        const QString wanted = qEnvironmentVariable("HYDRUS_QT_IMPORT_OPTIONS_TAB");

        for (int index = 0; index < editor.tabs->count(); ++index) {
            if (editor.tabs->widget(index)->property("option_type").toString() == wanted) {
                editor.tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    }
}

void HydrusImportOptionsDialog::rebuild_list(const QString &select) {
    rebuilding = true;

    const QString wanted = select.isEmpty() ? selected_key : select;
    list->clear();

    const QJsonArray callers = data.value(QStringLiteral("caller_types")).toArray();

    for (const QJsonValue &value : callers) {
        const QJsonObject caller = value.toObject();
        auto *item = new QListWidgetItem(
            QStringLiteral("%1  (default)").arg(caller.value(QStringLiteral("label")).toString()),
            list);
        item->setData(Qt::UserRole, QStringLiteral("default:%1").arg(caller.value(QStringLiteral("key")).toString()));
    }

    const QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();

    for (auto iterator = favourites.constBegin(); iterator != favourites.constEnd(); ++iterator) {
        auto *item = new QListWidgetItem(
            QStringLiteral("favourite: %1").arg(iterator.key()),
            list);
        item->setData(Qt::UserRole, QStringLiteral("favourite:%1").arg(iterator.key()));
    }

    int row_to_select = 0;

    for (int row = 0; row < list->count(); ++row) {
        if (list->item(row)->data(Qt::UserRole).toString() == wanted) {
            row_to_select = row;
            break;
        }
    }

    rebuilding = false;

    if (list->count() > 0) {
        list->setCurrentRow(row_to_select);
    }
}

QString HydrusImportOptionsDialog::current_key() const {
    const QListWidgetItem *item = list->currentItem();

    if (item == nullptr) {
        return QString();
    }

    return item->data(Qt::UserRole).toString();
}

void HydrusImportOptionsDialog::store_current() {
    if (selected_key.isEmpty()) {
        return;
    }

    QJsonObject defaults = data.value(QStringLiteral("defaults")).toObject();
    QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();
    const QJsonObject container = read_editor();

    if (selected_key.startsWith(QStringLiteral("default:"))) {
        defaults.insert(selected_key.mid(8), container);
        data.insert(QStringLiteral("defaults"), defaults);
    } else {
        favourites.insert(selected_key.mid(10), container);
        data.insert(QStringLiteral("favourites"), favourites);
    }
}

void HydrusImportOptionsDialog::add_favourite() {
    store_current();

    QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();
    QJsonObject defaults = data.value(QStringLiteral("defaults")).toObject();
    QJsonObject container;

    if (selected_key.startsWith(QStringLiteral("default:"))) {
        container = defaults.value(selected_key.mid(8)).toObject();
    } else if (selected_key.startsWith(QStringLiteral("favourite:"))) {
        container = favourites.value(selected_key.mid(10)).toObject();
    }

    QString base = QStringLiteral("new import options");
    QString name = base;
    int counter = 2;

    while (favourites.contains(name)) {
        name = QStringLiteral("%1 (%2)").arg(base).arg(counter);
        counter += 1;
    }

    favourites.insert(name, container);
    data.insert(QStringLiteral("favourites"), favourites);
    rebuild_list(QStringLiteral("favourite:%1").arg(name));
}

void HydrusImportOptionsDialog::rename_favourite() {
    if (selected_key.startsWith(QStringLiteral("default:"))) {
        return;
    }

    store_current();

    const QString old_name = selected_key.mid(10);
    bool ok = false;
    const QString new_name = QInputDialog::getText(
        this,
        QStringLiteral("rename import options"),
        QStringLiteral("new name:"),
        QLineEdit::Normal,
        old_name,
        &ok).trimmed();

    if (!ok || new_name.isEmpty() || new_name == old_name) {
        return;
    }

    QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();

    if (favourites.contains(new_name)) {
        QMessageBox::warning(this, QStringLiteral("rename import options"), QStringLiteral("that name already exists"));
        return;
    }

    const QJsonObject container = favourites.take(old_name).toObject();
    favourites.insert(new_name, container);
    data.insert(QStringLiteral("favourites"), favourites);
    selected_key = QStringLiteral("favourite:%1").arg(new_name);
    rebuild_list(selected_key);
}

void HydrusImportOptionsDialog::delete_favourite() {
    if (selected_key.startsWith(QStringLiteral("default:"))) {
        return;
    }

    const QString name = selected_key.mid(10);

    if (QMessageBox::question(
            this,
            QStringLiteral("delete import options"),
            QStringLiteral("delete the favourite container \"%1\"?").arg(name))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject favourites = data.value(QStringLiteral("favourites")).toObject();
    favourites.remove(name);
    data.insert(QStringLiteral("favourites"), favourites);
    selected_key = QStringLiteral("default:global");
    rebuild_list(selected_key);
}

void HydrusImportOptionsDialog::save() {
    if (specific_mode) {
        const QJsonObject specific = data.value(QStringLiteral("specific")).toObject();
        QJsonObject payload;
        payload.insert(QStringLiteral("target"), specific.value(QStringLiteral("target")));
        payload.insert(QStringLiteral("caller"), specific.value(QStringLiteral("caller")));
        payload.insert(QStringLiteral("container"), read_editor());

        main->notify(
            specific.value(QStringLiteral("action")).toString(),
            QString(),
            json_payload(payload));

        close();
        return;
    }

    store_current();

    QJsonObject payload;
    payload.insert(QStringLiteral("defaults"), data.value(QStringLiteral("defaults")));
    payload.insert(QStringLiteral("favourites"), data.value(QStringLiteral("favourites")));

    main->notify(QStringLiteral("import_options_save"), QString(), json_payload(payload));

    close();
}

void HydrusMainWindow::show_import_options(const QJsonObject &state) {
    if (import_options_dialog != nullptr) {
        import_options_dialog->close();
    }

    import_options_dialog = new HydrusImportOptionsDialog(this, state, this);
    connect(import_options_dialog, &QObject::destroyed, this, [this]() { import_options_dialog = nullptr; });

    import_options_dialog->show();
    import_options_dialog->raise();
    import_options_dialog->activateWindow();
}
