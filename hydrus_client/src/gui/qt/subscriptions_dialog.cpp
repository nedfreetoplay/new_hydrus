#include "bridge.hpp"

#include "support/support.hpp"

#include <QApplication>
#include <QClipboard>
#include <QComboBox>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QPushButton>
#include <QSize>
#include <QSpinBox>
#include <QTableWidget>
#include <QToolButton>
#include <QUrl>
#include <QVBoxLayout>

// WS-15.1.5: HydrusSubscriptionsDialog moved verbatim from bridge.cpp (move-only split).

HydrusSubscriptionsDialog::HydrusSubscriptionsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage subscriptions"));
    setObjectName(QStringLiteral("manage_subscriptions_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(1180, 460);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(3);

    auto *help_row = new QHBoxLayout();
    help_row->setContentsMargins(0, 0, 0, 0);
    help_row->setSpacing(4);
    help_row->addStretch(1);

    auto *help_text = new QLabel(QStringLiteral("help for this panel -->"), this);
    help_text->setObjectName(QStringLiteral("HydrusIndeterminate"));
    help_row->addWidget(help_text);

    auto *help_button = new QToolButton(this);
    help_button->setIcon(hydrus_small_icon(QStringLiteral("help")));
    help_button->setIconSize(QSize(16, 16));
    help_button->setFixedSize(22, 22);
    help_button->setAutoRaise(true);
    help_button->setFocusPolicy(Qt::NoFocus);
    help_button->setToolTip(QStringLiteral("open the html subscriptions help"));
    help_row->addWidget(help_button);
    layout->addLayout(help_row);

    connect(help_button, &QToolButton::clicked, this, []() {
        QDesktopServices::openUrl(QUrl(QStringLiteral(
            "https://hydrusnetwork.github.io/hydrus/help/getting_started_subscriptions.html")));
    });

    warning = new QLabel(
        QStringLiteral("SUBSCRIPTIONS ARE CURRENTLY GLOBALLY PAUSED! CHECK THE NETWORK MENU TO "
                       "UNPAUSE THEM."),
        this);
    warning->setObjectName(QStringLiteral("HydrusWarning"));
    warning->setWordWrap(true);
    warning->hide();
    layout->addWidget(warning);

    table = new QTableWidget(this);
    table->setColumnCount(9);
    table->setHorizontalHeaderLabels({QStringLiteral("name"),
                                      QStringLiteral("source"),
                                      QStringLiteral("status"),
                                      QStringLiteral("last new file time"),
                                      QStringLiteral("last checked"),
                                      QStringLiteral("error/delay?"),
                                      QStringLiteral("items"),
                                      QStringLiteral("paused"),
                                      QStringLiteral("custom import options")});
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    table->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);
    table->setMinimumHeight(120);

    const int column_widths[] = {150, 120, 120, 130, 110, 100, 70, 60, 260};

    for (int index = 0; index < 9; ++index) {
        table->setColumnWidth(index, column_widths[index]);
        table->horizontalHeader()->setSectionResizeMode(
            index, index == 8 ? QHeaderView::Stretch : QHeaderView::Interactive);
    }

    layout->addWidget(table, 1);

    const auto make_row = [this, layout]() {
        auto *row = new QWidget(this);
        auto *row_layout = new QHBoxLayout(row);
        row_layout->setContentsMargins(0, 0, 0, 0);
        row_layout->setSpacing(4);
        row_layout->addStretch(1);
        layout->addWidget(row);
        return row_layout;
    };

    const auto add_button = [this](QHBoxLayout *row, const QString &text) {
        auto *button = new QPushButton(text, this);
        button->setMinimumHeight(22);
        row->addWidget(button);
        return button;
    };

    const auto add_tool = [this](QHBoxLayout *row,
                                 const QString &icon_kind,
                                 const QString &tooltip) {
        auto *button = new QToolButton(this);
        button->setIcon(hydrus_small_icon(icon_kind));
        button->setIconSize(QSize(16, 16));
        button->setFixedSize(28, 24);
        button->setAutoRaise(true);
        button->setFocusPolicy(Qt::NoFocus);
        button->setToolTip(tooltip);
        row->addWidget(button);
        return button;
    };

    QHBoxLayout *row1 = make_row();
    auto *add = add_button(row1, QStringLiteral("add"));
    auto *edit_button = add_button(row1, QStringLiteral("edit"));
    auto *remove = add_button(row1, QStringLiteral("delete"));
    auto *export_button = add_button(row1, QStringLiteral("export"));
    auto *import_button = add_button(row1, QStringLiteral("import"));
    auto *duplicate = add_button(row1, QStringLiteral("duplicate"));
    selection_buttons << edit_button << remove << export_button << duplicate;

    QHBoxLayout *row2 = make_row();
    auto *merge = add_button(row2, QStringLiteral("merge"));
    auto *separate = add_button(row2, QStringLiteral("separate"));
    auto *deduplicate = add_button(row2, QStringLiteral("deduplicate"));
    auto *lowercase = add_button(row2, QStringLiteral("lowercase"));
    selection_buttons << merge << separate << deduplicate << lowercase;

    QHBoxLayout *row3 = make_row();
    auto *pause_button = add_button(row3, QStringLiteral("pause/resume"));
    auto *retry = add_tool(row3, QStringLiteral("retry"), QStringLiteral("retry commands"));
    auto *scrub = add_button(row3, QStringLiteral("scrub delays"));
    auto *check_now = add_button(row3, QStringLiteral("check queries now"));
    auto *reset_button = add_button(row3, QStringLiteral("reset"));
    selection_buttons << pause_button << scrub << check_now << reset_button;

    QHBoxLayout *row4 = make_row();
    auto *select_button = add_button(row4, QStringLiteral("select subscriptions"));
    auto *overwrite_downloader = add_button(row4, QStringLiteral("overwrite downloader"));
    auto *overwrite_checker = add_button(row4, QStringLiteral("overwrite checker options"));
    selection_buttons << overwrite_downloader << overwrite_checker;

    QHBoxLayout *row5 = make_row();
    auto *import_options_label = new QLabel(QStringLiteral("import options:"), this);
    row5->addWidget(import_options_label);
    auto *copy_button = add_tool(row5, QStringLiteral("copy"), QStringLiteral("copy import options"));
    auto *paste_button =
        add_tool(row5, QStringLiteral("paste"), QStringLiteral("paste a new set of options"));
    auto *favourites = add_tool(
        row5, QStringLiteral("star"), QStringLiteral("import options favourites"));
    auto *clear_options = add_button(row5, QStringLiteral("clear"));
    selection_buttons << copy_button << paste_button << clear_options;

    auto *bottom = new QHBoxLayout();
    bottom->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), this);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    bottom->addWidget(apply);
    bottom->addWidget(cancel);
    layout->addLayout(bottom);

    connect(add, &QPushButton::clicked, this, [this]() { add_subscription(); });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_edit"));
    });
    connect(remove, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_delete"));
    });
    connect(export_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("subscription export is not implemented yet"));
    });
    connect(import_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("subscription import is not implemented yet"));
    });
    connect(duplicate, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_duplicate"));
    });
    connect(merge, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("nothing to merge: subscriptions already have one downloader"));
    });
    connect(separate, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("this subscription has only one query"));
    });
    connect(deduplicate, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("no duplicate queries to fix"));
    });
    connect(lowercase, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_lowercase_queries"));
    });
    connect(pause_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_toggle"));
    });
    connect(retry, &QToolButton::clicked, this, [this, retry]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("retry ignored"), this, [this]() {
            main->notify(QStringLiteral("notify"),
                         QString(),
                         QStringLiteral("there are no ignored files to retry"));
        });
        menu->addAction(QStringLiteral("retry failed"), this, [this]() {
            main->notify(QStringLiteral("notify"),
                         QString(),
                         QStringLiteral("there are no failed files to retry"));
        });
        menu->popup(retry->mapToGlobal(QPoint(0, retry->height())));
    });
    connect(scrub, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_scrub_delays"));
    });
    connect(check_now, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_check"));
    });
    connect(reset_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_reset"));
    });
    connect(select_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("select subscriptions is not implemented yet"));
    });
    connect(overwrite_downloader, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("overwrite downloader is not implemented yet"));
    });
    connect(overwrite_checker, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("overwrite checker options is not implemented yet"));
    });
    connect(copy_button, &QToolButton::clicked, this, [this]() {
        const QStringList names = selected_names();

        if (names.isEmpty()) {
            return;
        }

        const QJsonArray subscriptions = data.value(QStringLiteral("subscriptions")).toArray();

        for (const QJsonValue &value : subscriptions) {
            const QJsonObject subscription = value.toObject();

            if (subscription.value(QStringLiteral("name")).toString() == names.first()) {
                const QJsonDocument document(
                    subscription.value(QStringLiteral("import_options")).toObject());
                QApplication::clipboard()->setText(
                    QString::fromUtf8(document.toJson(QJsonDocument::Compact)));
                break;
            }
        }
    });
    connect(paste_button, &QToolButton::clicked, this, [this, paste_button]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("merge-paste"), this, [this]() {
            import_options_action(QStringLiteral("paste"));
        });
        menu->addAction(QStringLiteral("fill-in-gaps-paste"), this, [this]() {
            import_options_action(QStringLiteral("paste"));
        });
        menu->addAction(QStringLiteral("replace-paste"), this, [this]() {
            import_options_action(QStringLiteral("paste"));
        });
        menu->popup(paste_button->mapToGlobal(QPoint(0, paste_button->height())));
    });
    connect(favourites, &QToolButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("import options favourites are not implemented yet"));
    });
    connect(clear_options, &QPushButton::clicked, this, [this]() {
        import_options_action(QStringLiteral("clear"));
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("subscription_edit"));
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() { refresh_buttons(); });

    set_state(state);
}

void HydrusSubscriptionsDialog::set_state(const QJsonObject &state) {
    data = state;
    warning->setVisible(state.value(QStringLiteral("subs_paused")).toBool());

    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }
    const QJsonArray subscriptions = state.value(QStringLiteral("subscriptions")).toArray();

    table->setSortingEnabled(false);
    table->setRowCount(subscriptions.size());

    for (int index = 0; index < subscriptions.size(); ++index) {
        const QJsonObject subscription = subscriptions.at(index).toObject();
        const QString name = subscription.value(QStringLiteral("name")).toString();
        const QStringList cells = {
            name,
            subscription.value(QStringLiteral("source")).toString(),
            subscription.value(QStringLiteral("pretty_status")).toString(),
            subscription.value(QStringLiteral("last_new_file_time")).toString(),
            subscription.value(QStringLiteral("last_checked")).toString(),
            subscription.value(QStringLiteral("error")).toString(),
            subscription.value(QStringLiteral("items")).toString(),
            subscription.value(QStringLiteral("paused_display")).toString(),
            subscription.value(QStringLiteral("custom_import_options")).toString(),
        };

        for (int column = 0; column < cells.size(); ++column) {
            auto *item = table->item(index, column);

            if (item == nullptr) {
                item = new QTableWidgetItem();
                table->setItem(index, column, item);
            }

            item->setText(cells[column]);
            item->setToolTip(cells[column]);
        }

        auto *name_item = table->item(index, 0);
        name_item->setData(Qt::UserRole, name);
        name_item->setToolTip(subscription.value(QStringLiteral("url")).toString());
    }

    table->setSortingEnabled(true);
    table->sortItems(0, Qt::AscendingOrder);

    if (!selected_name.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 0)->data(Qt::UserRole).toString() == selected_name) {
                table->selectRow(row);
                break;
            }
        }
    }

    refresh_buttons();
}

QStringList HydrusSubscriptionsDialog::selected_names() const {
    QStringList names;

    const QModelIndexList rows = table->selectionModel()->selectedRows();

    for (const QModelIndex &index : rows) {
        QTableWidgetItem *item = table->item(index.row(), 0);

        if (item != nullptr) {
            names.append(item->data(Qt::UserRole).toString());
        }
    }

    return names;
}

void HydrusSubscriptionsDialog::with_selected(const QString &action) {
    const QStringList names = selected_names();

    for (const QString &name : names) {
        main->notify(action, QString(), name);
    }
}

void HydrusSubscriptionsDialog::import_options_action(const QString &action) {
    const QStringList names = selected_names();

    for (const QString &name : names) {
        if (action == QStringLiteral("clear")) {
            main->notify(
                QStringLiteral("subscription_import_options_clear"), QString(), name);
            continue;
        }

        const QString text = QApplication::clipboard()->text().trimmed();
        const QJsonDocument document = QJsonDocument::fromJson(text.toUtf8());

        if (!document.isObject()) {
            continue;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("target"), name);
        payload.insert(QStringLiteral("container"), document.object());
        main->notify(QStringLiteral("subscription_import_options_save"),
                     QString(),
                     json_payload(payload));
    }
}

void HydrusSubscriptionsDialog::refresh_buttons() {
    const bool any = !selected_names().isEmpty();

    for (QWidget *button : selection_buttons) {
        button->setEnabled(any);
    }
}

void HydrusSubscriptionsDialog::add_subscription() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add subscription"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *url = new QLineEdit(&dialog);
    auto *query = new QLineEdit(&dialog);
    auto *site = new QComboBox(&dialog);
    auto *filter = new QLineEdit(&dialog);
    auto *period = new QSpinBox(&dialog);
    period->setRange(60, 31536000);
    period->setValue(86400);

    site->addItem(QStringLiteral("auto detect"), QString());

    for (const QJsonValue &value : data.value(QStringLiteral("sites")).toArray()) {
        const QJsonObject parser = value.toObject();
        site->addItem(parser.value(QStringLiteral("title")).toString(),
                      parser.value(QStringLiteral("id")).toString());
    }

    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
    form->addRow(QStringLiteral("site parser"), site);
    form->addRow(QStringLiteral("link filter"), filter);
    form->addRow(QStringLiteral("period (seconds)"), period);

    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text());
    payload.insert(QStringLiteral("url"), url->text());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());

    main->notify(QStringLiteral("subscription_add"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_subscriptions_dialog(const QJsonObject &state) {
    if (subscriptions_dialog == nullptr) {
        subscriptions_dialog = new HydrusSubscriptionsDialog(this, state, this);
        connect(subscriptions_dialog, &QObject::destroyed, this, [this]() {
            subscriptions_dialog = nullptr;
        });
    } else {
        subscriptions_dialog->set_state(state);
    }

    subscriptions_dialog->show();
    subscriptions_dialog->raise();
    subscriptions_dialog->activateWindow();
}

void HydrusMainWindow::update_subscriptions_dialog(const QJsonObject &state) {
    if (subscriptions_dialog != nullptr) {
        subscriptions_dialog->set_state(state);
    }
}
