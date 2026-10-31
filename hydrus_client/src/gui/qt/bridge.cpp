#include "bridge.hpp"
#include "support/support.hpp"
#include "screens.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QCompleter>
#include <QCoreApplication>
#include <QDebug>
#include <QDateTime>
#include <QDateTimeEdit>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QDoubleSpinBox>
#include <QDrag>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QIcon>
#include <QHash>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QMimeData>
#include <QMovie>
#include <QMouseEvent>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPixmap>
#include <QPixmapCache>
#include <QPolygonF>
#include <QPushButton>
#include <QRegularExpression>
#include <QScreen>
#include <QScrollArea>
#include <QScrollBar>
#include <QSettings>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSize>
#include <QSlider>
#include <QSpinBox>
#include <QSplitter>
#include <QStringListModel>
#include <QStackedWidget>
#include <QStatusBar>
#include <QStyle>
#include <QStyleFactory>
#include <QSystemTrayIcon>
#include <QTabBar>
#include <QTableWidget>
#include <QTextEdit>
#include <QTimer>
#include <QRandomGenerator>
#include <QSet>
#include <QStandardPaths>
#include <QToolButton>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <functional>

// ---------------------------------------------------------------------------
// HydrusSubscriptionsPage
// ---------------------------------------------------------------------------

HydrusSubscriptionsPage::HydrusSubscriptionsPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("period"),
        QStringLiteral("last check"),
        QStringLiteral("next check"),
        QStringLiteral("imported"),
        QStringLiteral("failed"),
        QStringLiteral("status")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add subscription..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *edit_button = new QPushButton(QStringLiteral("edit..."), this);
    auto *toggle = new QPushButton(QStringLiteral("pause / resume"), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    auto *results = new QPushButton(QStringLiteral("open files"), this);
    import_options_button = new QPushButton(QStringLiteral("import options..."), this);
    buttons->addWidget(add);
    buttons->addWidget(check);
    buttons->addWidget(edit_button);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addWidget(results);
    buttons->addWidget(import_options_button);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_subscription(); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_check")); });
    connect(edit_button, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_edit")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_delete")); });
    connect(results, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("subscription_open_results")); });
    connect(import_options_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("subscription_import_options"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("subscription_edit"));
    });

    update_state(state);
}

const QString &HydrusSubscriptionsPage::page_id() const {
    return id;
}

void HydrusSubscriptionsPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0) {
        return;
    }

    QTableWidgetItem *item = table->item(row, 0);

    if (item != nullptr) {
        main->notify(action, id, item->data(Qt::UserRole).toString());
    }
}

void HydrusSubscriptionsPage::add_subscription() {
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

    for (const QJsonValue &value : sites) {
        const QJsonObject parser = value.toObject();
        site->addItem(parser.value(QStringLiteral("title")).toString(), parser.value(QStringLiteral("id")).toString());
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

    main->notify(QStringLiteral("subscription_add"), id, json_payload(payload));
}

void HydrusSubscriptionsPage::update_state(const QJsonObject &state) {
    sites = state.value(QStringLiteral("sites")).toArray();
    const QJsonArray subscriptions = state.value(QStringLiteral("subscriptions")).toArray();

    status->setText(QStringLiteral("%1 subscriptions").arg(subscriptions.size()));
    table->setRowCount(subscriptions.size());

    for (int index = 0; index < subscriptions.size(); ++index) {
        const QJsonObject subscription = subscriptions.at(index).toObject();
        const QString name = subscription.value(QStringLiteral("name")).toString();

        for (int column = 0; column < table->columnCount(); ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(name);
        name_item->setData(Qt::UserRole, name);
        name_item->setToolTip(subscription.value(QStringLiteral("url")).toString());

        table->item(index, 1)->setText(QStringLiteral("%1 s").arg(subscription.value(QStringLiteral("period")).toInt()));
        table->item(index, 2)->setText(subscription.value(QStringLiteral("last_check")).toInt() > 0
            ? QDateTime::fromSecsSinceEpoch(subscription.value(QStringLiteral("last_check")).toInt()).toString(QStringLiteral("yyyy-MM-dd hh:mm"))
            : QStringLiteral("never"));
        table->item(index, 3)->setText(subscription.value(QStringLiteral("running")).toBool()
            ? QStringLiteral("now")
            : QStringLiteral("in %1 s").arg(subscription.value(QStringLiteral("due_in")).toInt()));
        table->item(index, 4)->setText(QString::number(subscription.value(QStringLiteral("imported")).toInt()));
        table->item(index, 5)->setText(QString::number(subscription.value(QStringLiteral("failed")).toInt()));
        table->item(index, 6)->setText(subscription.value(QStringLiteral("status")).toString());

        if (subscription.value(QStringLiteral("paused")).toBool()) {
            table->item(index, 6)->setText(QStringLiteral("paused - ") + table->item(index, 6)->text());
        }
    }

    table->resizeRowsToContents();
}

HydrusSubscriptionEditorDialog::HydrusSubscriptionEditorDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("subscription editor"));
    setObjectName(QStringLiteral("hydrus_subscription_editor"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(620, 380);

    auto *layout = new QVBoxLayout(this);
    auto *form = new QFormLayout();

    name = new QLineEdit(this);
    url = new QLineEdit(this);
    url->setPlaceholderText(QStringLiteral("https://example.com/search?q={query}"));
    query = new QLineEdit(this);
    query->setPlaceholderText(QStringLiteral("search terms for {query}"));
    site = new QComboBox(this);
    filter = new QLineEdit(this);
    filter->setPlaceholderText(QStringLiteral("optional link filter"));
    period = new QSpinBox(this);
    period->setRange(60, 31536000);
    period->setSingleStep(300);
    paused = new QCheckBox(QStringLiteral("paused"), this);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("gallery url"), url);
    form->addRow(QStringLiteral("search terms"), query);
    form->addRow(QStringLiteral("site parser"), site);
    form->addRow(QStringLiteral("link filter"), filter);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QString(), paused);
    layout->addLayout(form);

    auto *help = new QLabel(
        QStringLiteral("Subscriptions use the page URL as a template; {query} is replaced with the "
                       "search terms. Import options are edited from the subscriptions page."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);
    layout->addStretch(1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    layout->addWidget(buttons);

    set_state(state);
}

void HydrusSubscriptionEditorDialog::set_state(const QJsonObject &state) {
    data = state;
    page_id = state.value(QStringLiteral("page_id")).toString();

    name->setText(state.value(QStringLiteral("name")).toString());
    url->setText(state.value(QStringLiteral("url")).toString());
    query->setText(state.value(QStringLiteral("query")).toString());
    filter->setText(state.value(QStringLiteral("filter")).toString());
    period->setValue(state.value(QStringLiteral("period")).toInt(86400));
    paused->setChecked(state.value(QStringLiteral("paused")).toBool());

    site->clear();
    site->addItem(QStringLiteral("auto detect"), QString());

    for (const QJsonValue &value : state.value(QStringLiteral("sites")).toArray()) {
        const QJsonObject parser = value.toObject();
        site->addItem(
            parser.value(QStringLiteral("title")).toString(),
            parser.value(QStringLiteral("id")).toString());
    }

    const int index = site->findData(state.value(QStringLiteral("site")).toString());

    if (index >= 0) {
        site->setCurrentIndex(index);
    }
}

void HydrusSubscriptionEditorDialog::save() {
    QJsonObject payload;
    payload.insert(QStringLiteral("original"), data.value(QStringLiteral("original")));
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("url"), url->text().trimmed());
    payload.insert(QStringLiteral("query"), query->text());
    payload.insert(QStringLiteral("site"), site->currentData().toString());
    payload.insert(QStringLiteral("filter"), filter->text());
    payload.insert(QStringLiteral("period"), period->value());
    payload.insert(QStringLiteral("paused"), paused->isChecked());

    main->notify(
        QStringLiteral("subscription_edit_save"),
        page_id,
        json_payload(payload));

    close();
}

void HydrusMainWindow::show_subscription_editor(const QJsonObject &state) {
    if (subscription_editor_dialog != nullptr) {
        subscription_editor_dialog->close();
    }

    subscription_editor_dialog = new HydrusSubscriptionEditorDialog(this, state, this);
    connect(subscription_editor_dialog, &QObject::destroyed, this, [this]() {
        subscription_editor_dialog = nullptr;
    });
    subscription_editor_dialog->show();
    subscription_editor_dialog->raise();
    subscription_editor_dialog->activateWindow();
}

// ---------------------------------------------------------------------------
// HydrusSubscriptionsDialog (network -> subscriptions...)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HydrusImportFoldersDialog (file -> manage import folders...)
// ---------------------------------------------------------------------------

HydrusImportFoldersDialog::HydrusImportFoldersDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("edit import folders"));
    setObjectName(QStringLiteral("edit_import_folders_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 260);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(6);

    auto *intro = new QLabel(
        QStringLiteral("Here you can set the client to regularly check certain folders for "
                       "new files to import."),
        this);
    intro->setWordWrap(true);
    layout->addWidget(intro);

    auto *warning = new QLabel(
        QStringLiteral("WARNING: Import folders check (and potentially move/delete!) the "
                       "contents of all subdirectories as well as the base directory!"),
        this);
    warning->setObjectName(QStringLiteral("HydrusWarning"));
    warning->setWordWrap(true);
    layout->addWidget(warning);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("name"),
                                      QStringLiteral("path"),
                                      QStringLiteral("paused"),
                                      QStringLiteral("check period")});
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    table->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);
    table->setColumnWidth(0, 200);
    table->setColumnWidth(1, 430);
    table->setColumnWidth(2, 70);
    table->setColumnWidth(3, 140);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->setMinimumHeight(120);
    table->setMaximumHeight(190);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto *add = new QPushButton(QStringLiteral("add"), this);
    edit_button = new QPushButton(QStringLiteral("edit"), this);
    delete_button = new QPushButton(QStringLiteral("delete"), this);
    add->setMinimumHeight(22);
    edit_button->setMinimumHeight(22);
    delete_button->setMinimumHeight(22);
    buttons->addWidget(add);
    buttons->addWidget(edit_button);
    buttons->addWidget(delete_button);
    layout->addLayout(buttons);

    auto *bottom = new QHBoxLayout();
    bottom->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), this);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    bottom->addWidget(apply);
    bottom->addWidget(cancel);
    layout->addLayout(bottom);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("import_folder_edit"), QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("import_folder_edit"));
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("import_folder_remove"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("import_folder_edit"));
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusImportFoldersDialog::set_state(const QJsonObject &state) {
    data = state;

    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();

    table->setSortingEnabled(false);
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();
        const QString name = folder.value(QStringLiteral("name")).toString();
        const QStringList cells = {
            name,
            folder.value(QStringLiteral("path")).toString(),
            folder.value(QStringLiteral("paused")).toBool() ? QStringLiteral("yes") : QString(),
            folder.value(QStringLiteral("pretty_period")).toString(),
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

        table->item(index, 0)->setData(Qt::UserRole, name);
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

void HydrusImportFoldersDialog::refresh_buttons() {
    const bool selected = table->currentRow() >= 0;
    edit_button->setEnabled(selected);
    delete_button->setEnabled(selected);
}

void HydrusImportFoldersDialog::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, QString(), table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusMainWindow::show_import_folders_dialog(const QJsonObject &state) {
    if (import_folders_dialog == nullptr) {
        import_folders_dialog = new HydrusImportFoldersDialog(this, state, this);
        connect(import_folders_dialog, &QObject::destroyed, this, [this]() {
            import_folders_dialog = nullptr;
        });
    } else {
        import_folders_dialog->set_state(state);
    }

    import_folders_dialog->show();
    import_folders_dialog->raise();
    import_folders_dialog->activateWindow();
}

void HydrusMainWindow::update_import_folders_dialog(const QJsonObject &state) {
    if (import_folders_dialog != nullptr) {
        import_folders_dialog->set_state(state);
    }
}

// ---------------------------------------------------------------------------
// HydrusExportFoldersDialog (file -> manage export folders...)
// ---------------------------------------------------------------------------

HydrusExportFoldersDialog::HydrusExportFoldersDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("edit export folders"));
    setObjectName(QStringLiteral("edit_export_folders_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(723, 170);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(4);

    auto *intro = new QLabel(
        QStringLiteral("Here you can set the client to regularly export a certain query to a "
                       "particular location."),
        this);
    layout->addWidget(intro);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({QStringLiteral("name"),
                                      QStringLiteral("path"),
                                      QStringLiteral("type"),
                                      QStringLiteral("query"),
                                      QStringLiteral("period"),
                                      QStringLiteral("phrase"),
                                      QStringLiteral("recent error?")});
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->setTextElideMode(Qt::ElideRight);
    table->setShowGrid(false);
    table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
    table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    table->horizontalHeader()->setMinimumSectionSize(20);
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);

    const int widths[] = {132, 20, 79, 106, 106, 132};

    for (int column = 0; column < 6; ++column) {
        table->setColumnWidth(column, widths[column]);
    }

    table->setMinimumHeight(98);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto *add = new QPushButton(QStringLiteral("add"), this);
    edit_button = new QPushButton(QStringLiteral("edit"), this);
    delete_button = new QPushButton(QStringLiteral("delete"), this);
    add->setMinimumSize(75, 24);
    edit_button->setMinimumSize(75, 24);
    delete_button->setMinimumSize(75, 24);
    buttons->addWidget(add);
    buttons->addWidget(edit_button);
    buttons->addWidget(delete_button);
    layout->addLayout(buttons);

    auto *bottom = new QHBoxLayout();
    bottom->addStretch(1);
    auto *apply = new QPushButton(QStringLiteral("apply"), this);
    apply->setObjectName(QStringLiteral("HydrusAccept"));
    apply->setMinimumSize(75, 24);
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    cancel->setObjectName(QStringLiteral("HydrusCancel"));
    cancel->setMinimumSize(75, 24);
    bottom->addWidget(apply);
    bottom->addWidget(cancel);
    layout->addLayout(bottom);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("export_folder_edit"), QString(), QString());
    });
    connect(edit_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("export_folder_edit"));
    });
    connect(delete_button, &QPushButton::clicked, this, [this]() {
        with_selected(QStringLiteral("export_folder_remove"));
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        with_selected(QStringLiteral("export_folder_edit"));
    });
    connect(table, &QTableWidget::itemSelectionChanged, this, [this]() {
        refresh_buttons();
    });
    auto *delete_shortcut = new QShortcut(QKeySequence::Delete, table);
    connect(delete_shortcut, &QShortcut::activated, this, [this]() {
        with_selected(QStringLiteral("export_folder_remove"));
    });
    connect(apply, &QPushButton::clicked, this, &QDialog::close);
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusExportFoldersDialog::set_state(const QJsonObject &state) {
    data = state;

    QString selected_name;

    if (table->currentRow() >= 0 && table->item(table->currentRow(), 0) != nullptr) {
        selected_name =
            table->item(table->currentRow(), 0)->data(Qt::UserRole).toString();
    }

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();

    table->setSortingEnabled(false);
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();
        const QString name = folder.value(QStringLiteral("name")).toString();
        const QStringList cells = {
            name,
            folder.value(QStringLiteral("path")).toString(),
            folder.value(QStringLiteral("pretty_type")).toString(),
            folder.value(QStringLiteral("query")).toString(),
            folder.value(QStringLiteral("pretty_period")).toString(),
            folder.value(QStringLiteral("phrase")).toString(),
            folder.value(QStringLiteral("last_error")).toString(),
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

        table->item(index, 0)->setData(Qt::UserRole, name);
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

void HydrusExportFoldersDialog::refresh_buttons() {
    const int selected = table->selectionModel()->selectedRows().size();
    edit_button->setEnabled(selected == 1);
    delete_button->setEnabled(selected >= 1);
}

void HydrusExportFoldersDialog::with_selected(const QString &action) {
    const QModelIndexList rows = table->selectionModel()->selectedRows();

    if (action == QStringLiteral("export_folder_edit") && rows.size() != 1) {
        return;
    }

    QStringList names;

    for (const QModelIndex &index : rows) {
        if (table->item(index.row(), 0) != nullptr) {
            names.append(table->item(index.row(), 0)->data(Qt::UserRole).toString());
        }
    }

    for (const QString &name : names) {
        main->notify(action, QString(), name);
    }
}

void HydrusMainWindow::show_export_folders_dialog(const QJsonObject &state) {
    if (export_folders_dialog == nullptr) {
        export_folders_dialog = new HydrusExportFoldersDialog(this, state, this);
        connect(export_folders_dialog, &QObject::destroyed, this, [this]() {
            export_folders_dialog = nullptr;
        });
    } else {
        export_folders_dialog->set_state(state);
    }

    export_folders_dialog->show();
    export_folders_dialog->raise();
    export_folders_dialog->activateWindow();
}

void HydrusMainWindow::update_export_folders_dialog(const QJsonObject &state) {
    if (export_folders_dialog != nullptr) {
        export_folders_dialog->set_state(state);
    }
}

// ---------------------------------------------------------------------------
// HydrusImportFoldersPage
// ---------------------------------------------------------------------------

HydrusImportFoldersPage::HydrusImportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(7);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("path"),
        QStringLiteral("check every"),
        QStringLiteral("after import"),
        QStringLiteral("filters"),
        QStringLiteral("status"),
        QStringLiteral("last queued")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add folder..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *check = new QPushButton(QStringLiteral("check now"), this);
    auto *toggle = new QPushButton(QStringLiteral("pause/resume"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(check);
    buttons->addWidget(toggle);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("import_folder_edit"), id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_edit")); });
    connect(check, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_check")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_toggle")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("import_folder_remove")); });

    update_state(state);
}

const QString &HydrusImportFoldersPage::page_id() const {
    return id;
}

void HydrusImportFoldersPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusImportFoldersPage::update_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();

        for (int column = 0; column < 7; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name = table->item(index, 0);
        name->setText(folder.value(QStringLiteral("name")).toString());
        name->setData(Qt::UserRole, folder.value(QStringLiteral("name")).toString());

        if (folder.value(QStringLiteral("paused")).toBool()) {
            name->setForeground(QColor(160, 160, 160));
        } else {
            name->setForeground(QBrush());
        }

        const QString action = folder.value(QStringLiteral("action")).toString();
        QString action_text = action;

        if (action == QStringLiteral("move")) {
            action_text = QStringLiteral("move -> %1").arg(
                folder.value(QStringLiteral("action_path")).toString());
        }

        QStringList filters;
        const QString extensions = folder.value(QStringLiteral("extensions")).toString().trimmed();
        const qint64 min_size = folder.value(QStringLiteral("min_size")).toVariant().toLongLong();
        const qint64 max_size = folder.value(QStringLiteral("max_size")).toVariant().toLongLong();
        const qint64 skip = folder.value(QStringLiteral("last_modified_skip")).toVariant().toLongLong();

        if (!extensions.isEmpty()) {
            filters.append(QStringLiteral("ext: %1").arg(extensions));
        }

        if (min_size > 0 || max_size > 0) {
            filters.append(QStringLiteral("%1 - %2 bytes").arg(min_size).arg(
                max_size > 0 ? QString::number(max_size) : QStringLiteral("any")));
        }

        if (skip > 0) {
            filters.append(QStringLiteral("skip files newer than %1 s").arg(skip));
        }

        table->item(index, 1)->setText(folder.value(QStringLiteral("path")).toString());
        table->item(index, 2)->setText(
            QStringLiteral("%1 s").arg(folder.value(QStringLiteral("period")).toInt()));
        table->item(index, 3)->setText(action_text);
        table->item(index, 4)->setText(filters.isEmpty() ? QStringLiteral("any") : filters.join(QStringLiteral("; ")));
        table->item(index, 5)->setText(folder.value(QStringLiteral("status")).toString());
        table->item(index, 6)->setText(QString::number(folder.value(QStringLiteral("last_imported")).toInt()));
    }

    table->resizeRowsToContents();
}

// ---------------------------------------------------------------------------
// HydrusExportFoldersPage
// ---------------------------------------------------------------------------

HydrusExportFoldersPage::HydrusExportFoldersPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(8);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("destination"),
        QStringLiteral("search"),
        QStringLiteral("filename"),
        QStringLiteral("mode"),
        QStringLiteral("sidecar"),
        QStringLiteral("status"),
        QStringLiteral("last exported")
    });
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add folder..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *run = new QPushButton(QStringLiteral("run now"), this);
    auto *review = new QPushButton(QStringLiteral("review files..."), this);
    auto *toggle = new QPushButton(QStringLiteral("pause/resume"), this);
    auto *reveal = new QPushButton(QStringLiteral("open folder"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(run);
    buttons->addWidget(review);
    buttons->addWidget(toggle);
    buttons->addWidget(reveal);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("export_folder_edit"), id);
    });
    connect(edit, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_edit")); });
    connect(run, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_run")); });
    connect(review, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_review")); });
    connect(toggle, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_toggle")); });
    connect(reveal, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_reveal")); });
    connect(remove, &QPushButton::clicked, this, [this]() { with_selected(QStringLiteral("export_folder_remove")); });

    update_state(state);
}

const QString &HydrusExportFoldersPage::page_id() const {
    return id;
}

void HydrusExportFoldersPage::with_selected(const QString &action) {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    main->notify(action, id, table->item(row, 0)->data(Qt::UserRole).toString());
}

void HydrusExportFoldersPage::update_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    const QJsonArray folders = state.value(QStringLiteral("folders")).toArray();
    table->setRowCount(folders.size());

    for (int index = 0; index < folders.size(); ++index) {
        const QJsonObject folder = folders.at(index).toObject();

        for (int column = 0; column < 8; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name = table->item(index, 0);
        name->setText(folder.value(QStringLiteral("name")).toString());
        name->setData(Qt::UserRole, folder.value(QStringLiteral("name")).toString());

        if (folder.value(QStringLiteral("paused")).toBool()) {
            name->setForeground(QColor(160, 160, 160));
        } else {
            name->setForeground(QBrush());
        }

        const QString query = folder.value(QStringLiteral("query")).toString().trimmed();
        const QString mode = folder.value(QStringLiteral("export_type")).toString();
        const QString sidecar = folder.value(QStringLiteral("sidecar_format")).toString();

        QString mode_text = mode == QStringLiteral("synchronise")
            ? QStringLiteral("synchronise")
            : QStringLiteral("copy");

        if (folder.value(QStringLiteral("delete_from_client")).toBool()) {
            mode_text += QStringLiteral(" + delete from client");
        }

        table->item(index, 1)->setText(folder.value(QStringLiteral("path")).toString());
        table->item(index, 2)->setText(query.isEmpty() ? QStringLiteral("everything") : query);
        table->item(index, 3)->setText(folder.value(QStringLiteral("phrase")).toString());
        table->item(index, 4)->setText(mode_text);
        table->item(index, 5)->setText(sidecar == QStringLiteral("none") ? QStringLiteral("-") : sidecar);
        table->item(index, 6)->setText(folder.value(QStringLiteral("status")).toString());
        table->item(index, 7)->setText(QString::number(folder.value(QStringLiteral("last_exported")).toInt()));
    }

    table->resizeRowsToContents();
}

void HydrusMainWindow::show_duplicates_rules(const QJsonObject &state) {
    if (duplicates_rules_dialog == nullptr) {
        duplicates_rules_dialog = new HydrusDuplicatesRulesDialog(this, state, this);
        connect(duplicates_rules_dialog, &QObject::destroyed, this, [this]() { duplicates_rules_dialog = nullptr; });
    } else {
        duplicates_rules_dialog->set_state(state);
        duplicates_rules_dialog->raise();
        duplicates_rules_dialog->activateWindow();
    }

    duplicates_rules_dialog->show();
    for (auto it = duplicates.begin(); it != duplicates.end(); ++it) {
        if (it.value() != nullptr) {
            it.value()->update_auto_resolution_rules(state);
        }
    }
}

void HydrusMainWindow::show_duplicate_merge_options(const QJsonObject &state) {
    if (merge_options_dialog == nullptr) {
        merge_options_dialog = new HydrusDuplicateMergeOptionsDialog(this, state, this);
        connect(merge_options_dialog, &QObject::destroyed, this, [this]() { merge_options_dialog = nullptr; });
    } else {
        merge_options_dialog->set_state(state);
        merge_options_dialog->raise();
        merge_options_dialog->activateWindow();
    }

    merge_options_dialog->show();
}

void HydrusMainWindow::show_duplicate_custom_action(const QJsonObject &state) {
    if (duplicate_custom_action_dialog != nullptr) {
        QObject::disconnect(duplicate_custom_action_dialog, &QObject::destroyed, this, nullptr);
        duplicate_custom_action_dialog->close();
        duplicate_custom_action_dialog = nullptr;
    }

    duplicate_custom_action_dialog = new HydrusDuplicateCustomActionDialog(this, state, this);
    connect(duplicate_custom_action_dialog, &QObject::destroyed, this, [this]() { duplicate_custom_action_dialog = nullptr; });
    duplicate_custom_action_dialog->show();
}

void HydrusMainWindow::update_auto_resolution_preview(const QJsonObject &state) {
    if (duplicates_rules_dialog != nullptr) {
        duplicates_rules_dialog->set_preview(state);
    }
}

void HydrusMainWindow::show_duplicate_review(const QJsonObject &state) {
    if (duplicate_review_dialog == nullptr) {
        duplicate_review_dialog = new HydrusDuplicateReviewDialog(this, state, this);
        connect(duplicate_review_dialog, &QObject::destroyed, this, [this]() { duplicate_review_dialog = nullptr; });
        duplicate_review_dialog->show();
    } else {
        duplicate_review_dialog->set_state(state);
        duplicate_review_dialog->raise();
        duplicate_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_duplicate_review(const QJsonObject &state) {
    if (duplicate_review_dialog != nullptr) {
        duplicate_review_dialog->set_state(state);
    }
}

// ---------------------------------------------------------------------------
// HydrusMainWindow
// ---------------------------------------------------------------------------

void hydrus_show_or_dialog(HydrusMainWindow &window, rust::Str page_id, rust::Str initial) {
    show_or_predicate_dialog(window, to_qstring(page_id), to_qstring(initial));
}

void hydrus_show_page_manager(HydrusMainWindow &window, rust::Str json) {
    HydrusPageManager manager(&window, parse_object(json), &window);
    manager.exec();
}

void hydrus_show_command_palette(HydrusMainWindow &window) {
    show_command_palette(&window);
}

void hydrus_prompt_text(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str title,
    rust::Str label,
    rust::Str initial) {
    bool ok = false;
    const QString text = QInputDialog::getText(
        &window,
        from_rust(title),
        from_rust(label),
        QLineEdit::Normal,
        from_rust(initial),
        &ok);

    if (ok) {
        window.notify(from_rust(action), QString(), text);
    }
}

void hydrus_confirm_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str title,
    rust::Str text) {
    const QMessageBox::StandardButton answer = QMessageBox::question(
        &window,
        from_rust(title),
        from_rust(text),
        QMessageBox::Yes | QMessageBox::No,
        QMessageBox::No);

    if (answer == QMessageBox::Yes) {
        window.notify(from_rust(action), from_rust(target));
    }
}

void HydrusMainWindow::show_tag_dialog(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("manage metadata"));
    dialog.resize(460, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new HydrusMetadataPanel(this, &dialog);
    panel->set_suggestions(suggestions());
    panel->set_state(state.value(QStringLiteral("metadata")).toObject());
    layout->addWidget(panel, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void hydrus_show_manage_tags(HydrusMainWindow &window, rust::Str json) {
    HydrusManageTagsDialog dialog(&window, parse_object(json), &window);
    dialog.exec();
}

void hydrus_show_timestamps(HydrusMainWindow &window, rust::Str json) {
    HydrusTimestampsDialog dialog(&window, parse_object(json), &window);
    dialog.exec();
}

void HydrusMainWindow::show_services(const QJsonObject &state) {
    if (services_dialog == nullptr) {
        services_dialog = new HydrusServicesDialog(this, state, this);
        connect(services_dialog, &QObject::destroyed, this, [this]() { services_dialog = nullptr; });
        services_dialog->show();
    } else {
        services_dialog->set_state(state);
        services_dialog->raise();
        services_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_services(const QJsonObject &state) {
    if (services_dialog != nullptr) {
        services_dialog->set_state(state);
    }
}

void HydrusMainWindow::show_network(const QJsonObject &state) {
    if (network_dialog == nullptr) {
        network_dialog = new HydrusNetworkDialog(this, state, this);
        connect(network_dialog, &QObject::destroyed, this, [this]() { network_dialog = nullptr; });
        network_dialog->show();
    } else {
        network_dialog->set_state(state);
        network_dialog->raise();
        network_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_network(const QJsonObject &state) {
    if (network_dialog != nullptr) {
        network_dialog->set_state(state);
    }
}

// ---------------------------------------------------------------------------
// Free C++ bridge functions
// ---------------------------------------------------------------------------

QApplication *new_application_impl() {
    static int argc = 1;
    static char name[] = "hydrus_client";
    static char *argv[] = {name, nullptr};
    return new QApplication(argc, argv);
}

std::unique_ptr<QApplication> new_application() {
    return std::unique_ptr<QApplication>(new_application_impl());
}

int exec_application(QApplication &application) {
    return application.exec();
}

void quit_application(QApplication &application) {
    application.quit();
}

void qapp_process_events() {
    QApplication::processEvents();
}

std::unique_ptr<QSplashScreen> new_hydrus_splash() {
    QPixmap pixmap(620, 340);
    pixmap.fill(QColor(36, 36, 36));

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);

    QFont title_font = painter.font();
    title_font.setPointSize(30);
    title_font.setBold(true);
    painter.setFont(title_font);
    painter.setPen(QColor(238, 238, 238));
    painter.drawText(QRect(36, 48, 548, 64), Qt::AlignLeft | Qt::AlignVCenter, QStringLiteral("Hydrus Network"));

    QFont subtitle_font = painter.font();
    subtitle_font.setPointSize(13);
    subtitle_font.setBold(false);
    painter.setFont(subtitle_font);
    painter.setPen(QColor(170, 170, 170));
    painter.drawText(
        QRect(38, 118, 548, 30),
        Qt::AlignLeft | Qt::AlignVCenter,
        QStringLiteral("Rust + Qt 6 client"));

    painter.setPen(QColor(120, 120, 120));
    painter.drawLine(38, 168, 582, 168);

    painter.setPen(QColor(150, 150, 150));
    painter.drawText(
        QRect(38, 186, 548, 80),
        Qt::AlignLeft | Qt::AlignTop,
        QStringLiteral("A media management application for the hydrus network.\n"
                       "Opening the client database and preparing the session..."));
    painter.end();

    auto *splash = new QSplashScreen(pixmap);
    splash->setWindowFlag(Qt::WindowStaysOnTopHint, true);
    splash->setWindowFlag(Qt::FramelessWindowHint, true);
    return std::unique_ptr<QSplashScreen>(splash);
}

void hydrus_splash_show(QSplashScreen &splash) {
    splash.show();
    QApplication::processEvents();
}

void hydrus_splash_message(QSplashScreen &splash, rust::Str message) {
    splash.showMessage(
        to_qstring(message),
        Qt::AlignLeft | Qt::AlignBottom,
        QColor(225, 225, 225));
    QApplication::processEvents();
}

void hydrus_splash_finish_later(QSplashScreen &splash, int delay_ms) {
    QTimer::singleShot(delay_ms, &splash, [&splash]() { splash.close(); });
}

void hydrus_splash_screenshot(QSplashScreen &splash, rust::Str path, int delay_ms) {
    const QString target = to_qstring(path);

    QTimer::singleShot(delay_ms, &splash, [&splash, target]() {
        splash.grab().save(target);
    });
}

void hydrus_splash_finish(QSplashScreen &splash) {
    splash.close();
}

std::unique_ptr<HydrusMainWindow> new_hydrus_main_window() {
    return std::make_unique<HydrusMainWindow>();
}

void build_hydrus_shell(HydrusMainWindow &window) {
    window.build_shell();
}

void show_hydrus_main_window(HydrusMainWindow &window) {
    window.show();
}

void hydrus_set_title(HydrusMainWindow &window, rust::Str title) {
    window.setWindowTitle(to_qstring(title));
}

void hydrus_resize_window(HydrusMainWindow &window, int width, int height) {
    window.resize(width, height);
}

void hydrus_set_menus(HydrusMainWindow &window, rust::Str json) {
    window.set_menus(parse_array(json));
}

void hydrus_set_tag_suggestions(HydrusMainWindow &window, rust::Str json) {
    window.set_tag_suggestions(json_to_strings(parse_array(json)));
}

void hydrus_show_autocomplete(HydrusMainWindow &window, rust::Str json) {
    window.show_autocomplete(parse_object(json));
}

void hydrus_set_status(HydrusMainWindow &window, rust::Str json) {
    window.set_status(parse_object(json));
}

void hydrus_build_pages(HydrusMainWindow &window, rust::Str json) {
    window.build_pages(parse_object(json));
}

void hydrus_update_page(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_page(to_qstring(page_id), parse_object(json));
}

void hydrus_update_page_progress(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_page_progress(to_qstring(page_id), parse_object(json));
}

void hydrus_update_downloader(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_downloader(to_qstring(page_id), parse_object(json));
}

void hydrus_update_subscriptions(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_subscriptions(to_qstring(page_id), parse_object(json));
}

void hydrus_show_subscriptions_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_subscriptions_dialog(parse_object(json));
}

void hydrus_update_subscriptions_dialog(HydrusMainWindow &window, rust::Str json) {
    window.update_subscriptions_dialog(parse_object(json));
}

void hydrus_show_subscription_editor(HydrusMainWindow &window, rust::Str json) {
    window.show_subscription_editor(parse_object(json));
}

void hydrus_update_duplicates(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_duplicates(to_qstring(page_id), parse_object(json));
}

void hydrus_update_import_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_import_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_show_import_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folders_dialog(parse_object(json));
}

void hydrus_update_import_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.update_import_folders_dialog(parse_object(json));
}

void hydrus_update_export_folders(HydrusMainWindow &window, rust::Str page_id, rust::Str json) {
    window.update_export_folders(to_qstring(page_id), parse_object(json));
}

void hydrus_show_export_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_export_folders_dialog(parse_object(json));
}

void hydrus_update_export_folders_dialog(HydrusMainWindow &window, rust::Str json) {
    window.update_export_folders_dialog(parse_object(json));
}

void hydrus_show_duplicates_rules(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicates_rules(parse_object(json));
}

void hydrus_show_duplicate_merge_options(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicate_merge_options(parse_object(json));
}

void hydrus_show_duplicate_custom_action(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicate_custom_action(parse_object(json));
}

void hydrus_show_import_options(HydrusMainWindow &window, rust::Str json) {
    window.show_import_options(parse_object(json));
}

void hydrus_show_file_seed_cache(HydrusMainWindow &window, rust::Str json) {
    window.show_file_seed_cache(parse_object(json));
}

void hydrus_show_gallery_seed_log(HydrusMainWindow &window, rust::Str json) {
    window.show_gallery_seed_log(parse_object(json));
}

void hydrus_update_auto_resolution_preview(HydrusMainWindow &window, rust::Str json) {
    window.update_auto_resolution_preview(parse_object(json));
}

void hydrus_show_duplicate_review(HydrusMainWindow &window, rust::Str json) {
    window.show_duplicate_review(parse_object(json));
}

void hydrus_update_duplicate_review(HydrusMainWindow &window, rust::Str json) {
    window.update_duplicate_review(parse_object(json));
}

void hydrus_show_tag_relations(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_relations(parse_object(json));
}

void hydrus_show_import_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_import_folder(parse_object(json));
}

void hydrus_show_local_import_review(HydrusMainWindow &window, rust::Str json) {
    window.show_local_import_review(parse_object(json));
}

void hydrus_show_local_import_tagging(HydrusMainWindow &window, rust::Str json) {
    window.show_local_import_tagging(parse_object(json));
}

void hydrus_close_local_import_review(HydrusMainWindow &window) {
    window.close_local_import_review();
}

void hydrus_show_repair_file_locations(HydrusMainWindow &window, rust::Str json) {
    window.show_repair_file_locations(parse_object(json));
}

void hydrus_close_repair_file_locations(HydrusMainWindow &window) {
    window.close_repair_file_locations();
}

void hydrus_show_export_folder(HydrusMainWindow &window, rust::Str json) {
    window.show_export_folder(parse_object(json));
}

void hydrus_show_export_review(HydrusMainWindow &window, rust::Str json) {
    window.show_export_review(parse_object(json));
}

void hydrus_show_notes(HydrusMainWindow &window, rust::Str json) {
    window.show_notes(parse_object(json));
}

void hydrus_set_selection(
    HydrusMainWindow &window,
    rust::Str page_id,
    rust::Str hash,
    rust::Str tags_json,
    rust::Str preview_path,
    rust::Str metadata_json) {
    window.set_selection(
        to_qstring(page_id),
        to_qstring(hash),
        json_to_strings(parse_array(tags_json)),
        to_qstring(preview_path),
        parse_object(metadata_json));
}

void hydrus_show_page_chooser(HydrusMainWindow &window, rust::Str notebook_id) {
    window.show_page_chooser(to_qstring(notebook_id));
}

void hydrus_pick_files(HydrusMainWindow &window) {
    const QStringList paths = QFileDialog::getOpenFileNames(
        &window,
        QStringLiteral("import files"),
        QString(),
        QStringLiteral("all files (*.*)"));

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    window.notify(QStringLiteral("import_paths"), QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
}

void hydrus_prompt_url(HydrusMainWindow &window) {
    window.prompt_url();
}

void hydrus_open_viewer(HydrusMainWindow &window, rust::Str json) {
    window.open_viewer(parse_object(json));
}

void hydrus_update_viewer(HydrusMainWindow &window, rust::Str viewer_id, rust::Str json) {
    window.update_viewer(to_qstring(viewer_id), parse_object(json));
}

void hydrus_close_viewer(HydrusMainWindow &window, rust::Str viewer_id) {
    window.close_viewer(to_qstring(viewer_id));
}

void hydrus_show_tag_dialog(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_dialog(parse_object(json));
}

void hydrus_show_notifications(HydrusMainWindow &window, rust::Str json) {
    window.show_notifications(parse_object(json));
}

void hydrus_show_viewing_stats(HydrusMainWindow &window, rust::Str json) {
    window.show_viewing_stats(parse_object(json));
}

void hydrus_show_about(HydrusMainWindow &window, rust::Str json) {
    window.show_about(parse_object(json));
}

void hydrus_export_downloader(HydrusMainWindow &window, rust::Str page_id) {
    const QString path = QFileDialog::getSaveFileName(
        &window,
        QStringLiteral("export downloader definition"),
        QStringLiteral("downloader.json"),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(QStringLiteral("downloader_export_to"), to_qstring(page_id), path);
    }
}

void hydrus_import_downloader(HydrusMainWindow &window, rust::Str target) {
    const QString path = QFileDialog::getOpenFileName(
        &window,
        QStringLiteral("import downloader definition"),
        QString(),
        QStringLiteral("JSON (*.json)"));

    if (!path.isEmpty()) {
        window.notify(QStringLiteral("downloader_import_from"), to_qstring(target), path);
    }
}

void hydrus_show_options(HydrusMainWindow &window, rust::Str json) {
    window.show_options(parse_object(json));
}

void hydrus_show_services(HydrusMainWindow &window, rust::Str json) {
    window.show_services(parse_object(json));
}

void hydrus_update_services(HydrusMainWindow &window, rust::Str json) {
    window.update_services(parse_object(json));
}

void hydrus_show_tag_filter(HydrusMainWindow &window, rust::Str json) {
    window.show_tag_filter(parse_object(json));
}

void hydrus_show_api_permissions(HydrusMainWindow &window, rust::Str json) {
    window.show_api_permissions(parse_object(json));
}

void hydrus_update_api_permissions(HydrusMainWindow &window, rust::Str json) {
    window.update_api_permissions(parse_object(json));
}

void hydrus_show_account_types(HydrusMainWindow &window, rust::Str json) {
    window.show_account_types(parse_object(json));
}

void hydrus_update_account_types(HydrusMainWindow &window, rust::Str json) {
    window.update_account_types(parse_object(json));
}

void hydrus_show_orphan_files(HydrusMainWindow &window, rust::Str json) {
    window.show_orphan_files(parse_object(json));
}

void hydrus_update_orphan_files(HydrusMainWindow &window, rust::Str json) {
    window.update_orphan_files(parse_object(json));
}

void hydrus_show_database_backup(HydrusMainWindow &window, rust::Str json) {
    window.show_database_backup(parse_object(json));
}

void hydrus_update_database_backup(HydrusMainWindow &window, rust::Str json) {
    window.update_database_backup(parse_object(json));
}

void hydrus_show_network(HydrusMainWindow &window, rust::Str json) {
    window.show_network(parse_object(json));
}

void hydrus_update_network(HydrusMainWindow &window, rust::Str json) {
    window.update_network(parse_object(json));
}

void hydrus_start_tick(HydrusMainWindow &window) {
    window.start_tick();
}

void hydrus_defer_action(
    HydrusMainWindow &window,
    rust::Str action,
    rust::Str target,
    rust::Str payload,
    int delay_ms) {
    const QString action_text = to_qstring(action);
    const QString target_text = to_qstring(target);
    const QString payload_text = to_qstring(payload);

    QTimer::singleShot(delay_ms, &window, [&window, action_text, target_text, payload_text]() {
        QByteArray action_bytes = action_text.toUtf8();
        QByteArray target_bytes = target_text.toUtf8();
        QByteArray payload_bytes = payload_text.toUtf8();

        app_action(
            window,
            rust::Str(action_bytes.constData(), action_bytes.size()),
            rust::Str(target_bytes.constData(), target_bytes.size()),
            rust::Str(payload_bytes.constData(), payload_bytes.size()));
    });
}

void hydrus_show_message(HydrusMainWindow &window, rust::Str title, rust::Str text) {
    window.show_message(to_qstring(title), to_qstring(text));
}

void hydrus_notify_message(HydrusMainWindow &window, rust::Str message) {
    window.notify_message(to_qstring(message));
}

void hydrus_set_clipboard(HydrusMainWindow &window, rust::Str text) {
    Q_UNUSED(window);
    QApplication::clipboard()->setText(to_qstring(text));
}

void hydrus_open_path(rust::Str path) {
    QDesktopServices::openUrl(QUrl::fromLocalFile(to_qstring(path)));
}

void hydrus_open_url(rust::Str url) {
    QDesktopServices::openUrl(QUrl(to_qstring(url)));
}

void hydrus_schedule_screenshot(HydrusMainWindow &window, rust::Str path) {
    window.schedule_screenshot(to_qstring(path));
}

void qapp_set_application_name(rust::Str name) {
    QApplication::setApplicationName(to_qstring(name));
}

void qapp_set_application_display_name(rust::Str name) {
    QApplication::setApplicationDisplayName(to_qstring(name));
}

void qapp_set_organization_name(rust::Str name) {
    QApplication::setOrganizationName(to_qstring(name));
}

void qapp_set_style(rust::Str name) {
    if (QStyle *style = QStyleFactory::create(to_qstring(name))) {
        QApplication::setStyle(style);
    }
}

void qapp_set_stylesheet(rust::Str stylesheet) {
    qApp->setStyleSheet(to_qstring(stylesheet));
}

void qapp_quit() {
    QCoreApplication::quit();
}

