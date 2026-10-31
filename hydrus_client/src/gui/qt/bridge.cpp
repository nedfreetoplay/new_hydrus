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

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial);

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
// Page chooser
// ---------------------------------------------------------------------------

class HydrusPageChooser : public QDialog {
public:
    explicit HydrusPageChooser(const QString &notebook_id, QWidget *parent = nullptr)
        : QDialog(parent), notebook(notebook_id) {
        setWindowTitle(QStringLiteral("new page"));
        resize(460, 320);

        auto *layout = new QVBoxLayout(this);
        stack = new QStackedWidget(this);
        layout->addWidget(stack, 1);

        build_home();
        build_search();
        build_download();
        build_special();

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Cancel, this);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

    QString result;

private:
    QWidget *make_page() {
        auto *page = new QWidget(stack);
        auto *layout = new QVBoxLayout(page);
        layout->setSpacing(8);
        stack->addWidget(page);
        page_layout = layout;
        return page;
    }

    QPushButton *add_button(const QString &text, const QString &type) {
        auto *button = new QPushButton(text, stack->currentWidget());
        button->setMinimumHeight(34);
        connect(button, &QPushButton::clicked, this, [this, type]() {
            result = type;
            accept();
        });
        page_layout->addWidget(button);
        return button;
    }

    QPushButton *add_nav(const QString &text, int target) {
        auto *button = new QPushButton(text, stack->currentWidget());
        connect(button, &QPushButton::clicked, this, [this, target]() { stack->setCurrentIndex(target); });
        page_layout->addWidget(button);
        return button;
    }

    void build_home() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("choose a page type"), stack->currentWidget()));
        add_nav(QStringLiteral("file search"), 1);
        add_nav(QStringLiteral("download"), 2);
        add_nav(QStringLiteral("special"), 3);
        page_layout->addStretch(1);
    }

    void build_search() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("file search"), stack->currentWidget()));
        add_button(QStringLiteral("my files"), QStringLiteral("query:my files"));
        add_button(QStringLiteral("all local files"), QStringLiteral("query:all local files"));
        add_button(QStringLiteral("trash"), QStringLiteral("query:trash"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    void build_download() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("download"), stack->currentWidget()));
        add_button(QStringLiteral("urls"), QStringLiteral("urls"));
        add_button(QStringLiteral("watcher"), QStringLiteral("watcher"));
        add_button(QStringLiteral("gallery"), QStringLiteral("gallery"));
        add_button(QStringLiteral("simple downloader"), QStringLiteral("simple"));
        add_button(QStringLiteral("subscriptions"), QStringLiteral("subscriptions"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    void build_special() {
        make_page();
        page_layout->addWidget(new QLabel(QStringLiteral("special"), stack->currentWidget()));
        add_button(QStringLiteral("page of pages"), QStringLiteral("notebook"));
        add_button(QStringLiteral("duplicates processing"), QStringLiteral("duplicates"));
        add_button(QStringLiteral("import folders"), QStringLiteral("import_folders"));
        add_nav(QStringLiteral("back"), 0);
        page_layout->addStretch(1);
    }

    QString notebook;
    QStackedWidget *stack = nullptr;
    QVBoxLayout *page_layout = nullptr;
};

// ---------------------------------------------------------------------------
// Options and manager dialogs
// ---------------------------------------------------------------------------

namespace {

struct OptionField {
    QString key;
    QString label;
    QString type;
    bool noneable = false;
    int min_value = -2000000000;
    int max_value = 2000000000;
    QString none_phrase;
    QCheckBox *boolean = nullptr;
    QSpinBox *integer = nullptr;
    QLineEdit *text = nullptr;
    QWidget *noneable_container = nullptr;
    QSpinBox *noneable_integer = nullptr;
    QCheckBox *noneable_checkbox = nullptr;
    QWidget *page = nullptr;
    QWidget *label_widget = nullptr;
    QWidget *input_widget = nullptr;
};

class HydrusOptionsDialog : public QDialog {
public:
    HydrusOptionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
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

private:
    void apply() {
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

    HydrusMainWindow *main = nullptr;
    QLineEdit *search = nullptr;
    QVector<OptionField> fields;
};

} // namespace







// ---------------------------------------------------------------------------
// HydrusTagRelationsDialog
// ---------------------------------------------------------------------------

HydrusTagRelationsDialog::HydrusTagRelationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("tag siblings and parents"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(720, 520);

    auto *layout = new QVBoxLayout(this);

    auto *service_row = new QHBoxLayout();
    service_row->addWidget(new QLabel(QStringLiteral("service"), this));
    services = new QComboBox(this);
    service_row->addWidget(services, 1);
    layout->addLayout(service_row);

    auto *help = new QLabel(
        QStringLiteral("Siblings redirect a bad tag onto its preferred spelling. Parents describe a "
                       "child -> parent hierarchy. The left column is the tag being changed; the right "
                       "column is what it points to."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    tabs = new QTabWidget(this);

    auto make_table = [this]() {
        auto *table = new QTableWidget(this);
        table->setColumnCount(2);
        table->setHorizontalHeaderLabels({QStringLiteral("tag"), QStringLiteral("points to")});
        table->horizontalHeader()->setStretchLastSection(true);
        table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::SingleSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        return table;
    };

    siblings = make_table();
    parents = make_table();
    tabs->addTab(siblings, QStringLiteral("siblings"));
    tabs->addTab(parents, QStringLiteral("parents"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add pair..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(services, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) {
        rebuild();
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_relation(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusTagRelationsDialog::set_state(const QJsonObject &state) {
    const bool first_load = data.isEmpty();
    data = state;

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    for (const QJsonValue &value : state.value(QStringLiteral("services")).toArray()) {
        const QJsonObject service = value.toObject();
        services->addItem(service.value(QStringLiteral("name")).toString(),
                          service.value(QStringLiteral("key")).toString());
    }

    if (services->count() == 0) {
        services->addItem(QStringLiteral("no local tag service"), QString());
    }

    if (!previous.isEmpty()) {
        const int index = services->findData(previous);

        if (index >= 0) {
            services->setCurrentIndex(index);
        }
    }

    blocker.unblock();

    if (first_load) {
        const QString tab = state.value(QStringLiteral("tab")).toString();
        tabs->setCurrentIndex(tab == QStringLiteral("parents") ? 1 : 0);
    }

    rebuild();
}

void HydrusTagRelationsDialog::rebuild() {
    siblings->setRowCount(0);
    parents->setRowCount(0);

    const QString key = services->currentData().toString();

    for (const QJsonValue &value : data.value(QStringLiteral("relations")).toArray()) {
        const QJsonObject relation = value.toObject();

        if (relation.value(QStringLiteral("service")).toString() != key) {
            continue;
        }

        const bool is_parent = relation.value(QStringLiteral("relation")).toString() == QStringLiteral("parents");
        QTableWidget *table = is_parent ? parents : siblings;
        const int row = table->rowCount();
        table->setRowCount(row + 1);

        auto *first = new QTableWidgetItem(relation.value(QStringLiteral("first")).toString());
        first->setData(Qt::UserRole, relation.value(QStringLiteral("first")).toString());
        auto *second = new QTableWidgetItem(relation.value(QStringLiteral("second")).toString());
        second->setData(Qt::UserRole, relation.value(QStringLiteral("second")).toString());

        table->setItem(row, 0, first);
        table->setItem(row, 1, second);
    }

    for (QTableWidget *table : {siblings, parents}) {
        table->resizeRowsToContents();
    }
}

QTableWidget *HydrusTagRelationsDialog::current_table() const {
    return tabs->currentIndex() == 1 ? parents : siblings;
}

QString HydrusTagRelationsDialog::current_relation() const {
    return tabs->currentIndex() == 1 ? QStringLiteral("parents") : QStringLiteral("siblings");
}

void HydrusTagRelationsDialog::add_relation() {
    const QString relation = current_relation();
    const bool is_parent = relation == QStringLiteral("parents");

    QDialog dialog(this);
    dialog.setWindowTitle(is_parent ? QStringLiteral("add tag parent") : QStringLiteral("add tag sibling"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *first = new QLineEdit(&dialog);
    auto *second = new QLineEdit(&dialog);
    first->setPlaceholderText(is_parent ? QStringLiteral("child tag") : QStringLiteral("bad tag"));
    second->setPlaceholderText(is_parent ? QStringLiteral("parent tag") : QStringLiteral("good tag"));
    form->addRow(is_parent ? QStringLiteral("child") : QStringLiteral("bad tag"), first);
    form->addRow(is_parent ? QStringLiteral("parent") : QStringLiteral("good tag"), second);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    if (first->text().trimmed().isEmpty() || second->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), relation);
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), first->text().trimmed());
    payload.insert(QStringLiteral("second"), second->text().trimmed());

    main->notify(QStringLiteral("tag_relations_add"), QString(), json_payload(payload));
}

void HydrusTagRelationsDialog::remove_selected() {
    QTableWidget *table = current_table();
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr || table->item(row, 1) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("relation"), current_relation());
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("first"), table->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("second"), table->item(row, 1)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("tag_relations_remove"), QString(), json_payload(payload));
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

HydrusSeedCacheDialog::HydrusSeedCacheDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, bool gallery_mode, QWidget *parent)
    : QDialog(parent), main(main_window), gallery(gallery_mode) {
    setWindowTitle(gallery ? QStringLiteral("gallery seed log") : QStringLiteral("file seed cache"));
    setObjectName(gallery ? QStringLiteral("hydrus_gallery_seed_log")
                          : QStringLiteral("hydrus_file_seed_cache"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(940, 560);

    auto *layout = new QVBoxLayout(this);

    subtitle = new QLabel(this);
    subtitle->setWordWrap(true);
    layout->addWidget(subtitle);

    auto *controls = new QHBoxLayout();
    filter = new QComboBox(this);
    filter->setObjectName(QStringLiteral("seed_cache_filter"));
    search = new QLineEdit(this);
    search->setPlaceholderText(QStringLiteral("filter by url or note"));
    search->setClearButtonEnabled(true);
    controls->addWidget(new QLabel(QStringLiteral("show:"), this));
    controls->addWidget(filter);
    controls->addWidget(search, 1);
    layout->addLayout(controls);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("seed_cache_table"));
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({QStringLiteral("status"), QStringLiteral("seed"),
                                      QStringLiteral("created"), QStringLiteral("modified"),
                                      QStringLiteral("note")});
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSortingEnabled(true);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    retry_button = new QPushButton(QStringLiteral("retry failed"), this);
    auto *remove_button = new QPushButton(QStringLiteral("remove"), this);
    auto *clear_finished = new QPushButton(QStringLiteral("clear finished"), this);
    auto *clear_all = new QPushButton(QStringLiteral("clear all"), this);
    auto *copy = new QPushButton(QStringLiteral("copy urls"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);

    buttons->addWidget(retry_button);
    buttons->addWidget(remove_button);
    buttons->addWidget(clear_finished);
    buttons->addWidget(clear_all);
    buttons->addWidget(copy);
    buttons->addStretch(1);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(filter, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) { rebuild(); });
    connect(search, &QLineEdit::textChanged, this, [this](const QString &) { rebuild(); });
    connect(retry_button, &QPushButton::clicked, this, [this]() { send_action(QStringLiteral("retry")); });
    connect(remove_button, &QPushButton::clicked, this, [this]() {
        if (selected_urls().isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select one or more seeds first."));
            return;
        }

        send_action(QStringLiteral("remove"));
    });
    connect(clear_finished, &QPushButton::clicked, this, [this]() {
        send_action(QStringLiteral("clear_finished"));
    });
    connect(clear_all, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, windowTitle(), QStringLiteral("Remove every seed in this log?"))
            == QMessageBox::Yes) {
            send_action(QStringLiteral("clear"));
        }
    });
    connect(copy, &QPushButton::clicked, this, [this]() {
        QStringList urls = selected_urls();

        if (urls.isEmpty()) {
            for (const QJsonValue &value : data.value(QStringLiteral("seeds")).toArray()) {
                urls << value.toObject().value(QStringLiteral("url")).toString();
            }
        }

        QApplication::clipboard()->setText(urls.join(QStringLiteral("\n")));
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusSeedCacheDialog::set_state(const QJsonObject &state) {
    data = state;

    const QString current = filter->currentData().toString();
    const QJsonArray statuses = state.value(QStringLiteral("status_filters")).toArray();

    QSignalBlocker blocker(filter);
    filter->clear();

    for (const QJsonValue &value : statuses) {
        const QJsonObject entry = value.toObject();
        filter->addItem(entry.value(QStringLiteral("label")).toString(),
                        entry.value(QStringLiteral("key")).toString());
    }

    const int index = filter->findData(current);
    filter->setCurrentIndex(index >= 0 ? index : 0);
    blocker.unblock();

    rebuild();
}

QStringList HydrusSeedCacheDialog::selected_urls() const {
    QStringList urls;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString url = item->data(Qt::UserRole).toString();

        if (!url.isEmpty() && !urls.contains(url)) {
            urls << url;
        }
    }

    return urls;
}

void HydrusSeedCacheDialog::rebuild() {
    const QString status_filter = filter->currentData().toString();
    const QString needle = search->text().trimmed();

    table->setSortingEnabled(false);
    table->setRowCount(0);

    const QJsonArray seeds = data.value(QStringLiteral("seeds")).toArray();

    for (const QJsonValue &value : seeds) {
        const QJsonObject seed = value.toObject();
        const QString status = seed.value(QStringLiteral("status")).toString();
        const QString url = seed.value(QStringLiteral("url")).toString();
        const QString note = seed.value(QStringLiteral("note")).toString();

        if (status_filter != QStringLiteral("all") && status != status_filter) {
            continue;
        }

        if (!needle.isEmpty()
            && !url.contains(needle, Qt::CaseInsensitive)
            && !note.contains(needle, Qt::CaseInsensitive)) {
            continue;
        }

        const int row = table->rowCount();
        table->insertRow(row);

        auto *status_item = new QTableWidgetItem(status);

        if (status == QStringLiteral("error") || status == QStringLiteral("failed")) {
            status_item->setForeground(QColor(220, 110, 110));
        } else if (status == QStringLiteral("done") || status == QStringLiteral("success")) {
            status_item->setForeground(QColor(120, 190, 120));
        } else if (status == QStringLiteral("working")) {
            status_item->setForeground(QColor(220, 200, 120));
        }

        table->setItem(row, 0, status_item);

        auto *url_item = new QTableWidgetItem(url);
        url_item->setData(Qt::UserRole, url);
        url_item->setToolTip(url);
        table->setItem(row, 1, url_item);

        auto timestamp = [](const QJsonValue &value) {
            const qint64 seconds = value.toVariant().toLongLong();

            if (seconds <= 0) {
                return QString();
            }

            return QDateTime::fromSecsSinceEpoch(seconds).toString(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        };

        table->setItem(row, 2, new QTableWidgetItem(timestamp(seed.value(QStringLiteral("created")))));
        table->setItem(row, 3, new QTableWidgetItem(timestamp(seed.value(QStringLiteral("modified")))));
        table->setItem(row, 4, new QTableWidgetItem(note));
    }

    table->setSortingEnabled(true);
    subtitle->setText(data.value(QStringLiteral("subtitle")).toString());

    const QJsonObject counts = data.value(QStringLiteral("counts")).toObject();
    const int failed = counts.value(QStringLiteral("failed")).toInt();
    retry_button->setText(failed > 0 ? QStringLiteral("retry failed (%1)").arg(failed)
                                     : QStringLiteral("retry failed"));
    retry_button->setEnabled(failed > 0 || !selected_urls().isEmpty());
}

void HydrusSeedCacheDialog::send_action(const QString &action) {
    QJsonObject payload;
    payload.insert(QStringLiteral("page"), data.value(QStringLiteral("page")));
    payload.insert(QStringLiteral("action"), action);

    QJsonArray urls;
    for (const QString &url : selected_urls()) {
        urls.append(url);
    }
    payload.insert(QStringLiteral("urls"), urls);

    main->notify(gallery ? QStringLiteral("gallery_seed_log_action")
                         : QStringLiteral("file_seed_cache_action"),
                 QString(),
                 json_payload(payload));
}

void HydrusMainWindow::show_file_seed_cache(const QJsonObject &state) {
    if (file_seed_cache_dialog == nullptr) {
        file_seed_cache_dialog = new HydrusSeedCacheDialog(this, state, false, this);
        connect(file_seed_cache_dialog, &QObject::destroyed, this, [this]() { file_seed_cache_dialog = nullptr; });
    } else {
        file_seed_cache_dialog->set_state(state);
        file_seed_cache_dialog->raise();
        file_seed_cache_dialog->activateWindow();
    }

    file_seed_cache_dialog->show();
}

void HydrusMainWindow::show_gallery_seed_log(const QJsonObject &state) {
    if (gallery_seed_log_dialog == nullptr) {
        gallery_seed_log_dialog = new HydrusSeedCacheDialog(this, state, true, this);
        connect(gallery_seed_log_dialog, &QObject::destroyed, this, [this]() { gallery_seed_log_dialog = nullptr; });
    } else {
        gallery_seed_log_dialog->set_state(state);
        gallery_seed_log_dialog->raise();
        gallery_seed_log_dialog->activateWindow();
    }

    gallery_seed_log_dialog->show();
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

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

class HydrusCommandPalette : public QDialog {
public:
    explicit HydrusCommandPalette(HydrusMainWindow *main_window, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("command palette"));
        resize(520, 420);

        auto *layout = new QVBoxLayout(this);
        filter = new QLineEdit(this);
        filter->setPlaceholderText(QStringLiteral("type a command..."));
        layout->addWidget(filter);

        list = new QListWidget(this);
        list->setSelectionMode(QAbstractItemView::SingleSelection);
        layout->addWidget(list, 1);

        const QList<QPair<QString, QString>> commands = {
            {QStringLiteral("new page..."), QStringLiteral("new_page")},
            {QStringLiteral("manage pages..."), QStringLiteral("page_manager")},
            {QStringLiteral("duplicate current page"), QStringLiteral("duplicate_page")},
            {QStringLiteral("back"), QStringLiteral("history_back")},
            {QStringLiteral("forward"), QStringLiteral("history_forward")},
            {QStringLiteral("refresh current page"), QStringLiteral("refresh_page")},
            {QStringLiteral("options..."), QStringLiteral("options")},
            {QStringLiteral("manage tags..."), QStringLiteral("manage_tags_active")},
            {QStringLiteral("import files..."), QStringLiteral("import_files")},
            {QStringLiteral("import url..."), QStringLiteral("import_url")},
            {QStringLiteral("import folders..."), QStringLiteral("new_import_folders")},
            {QStringLiteral("manage export folders\u2026"), QStringLiteral("manage_export_folders")},
            {QStringLiteral("run all export folders"), QStringLiteral("export_folder_run_all")},
            {QStringLiteral("check all import folders"), QStringLiteral("import_folder_check_all")},
            {QStringLiteral("manage services..."), QStringLiteral("manage_services")},
            {QStringLiteral("client api permissions..."), QStringLiteral("api_permissions")},
            {QStringLiteral("clear orphan files..."), QStringLiteral("orphan_files_scan")},
            {QStringLiteral("review duplicate pending decisions..."), QStringLiteral("duplicates_review")},
            {QStringLiteral("back up database..."), QStringLiteral("db_backup_open")},
            {QStringLiteral("manage network..."), QStringLiteral("options_network")},
            {QStringLiteral("save session as..."), QStringLiteral("session_save_as")},
            {QStringLiteral("backup session"), QStringLiteral("session_backup")},
            {QStringLiteral("check database"), QStringLiteral("db_check")},
            {QStringLiteral("analyze database"), QStringLiteral("db_analyze")},
            {QStringLiteral("vacuum database"), QStringLiteral("db_vacuum")},
            {QStringLiteral("database statistics"), QStringLiteral("db_stats")},
            {QStringLiteral("undo closed page"), QStringLiteral("undo")},
            {QStringLiteral("documentation"), QStringLiteral("docs")},
            {QStringLiteral("about"), QStringLiteral("about")},
            {QStringLiteral("exit"), QStringLiteral("exit")},
        };

        for (const auto &[label, action] : commands) {
            auto *item = new QListWidgetItem(label, list);
            item->setData(Qt::UserRole, action);
        }

        connect(filter, &QLineEdit::textChanged, this, [this](const QString &text) {
            for (int index = 0; index < list->count(); ++index) {
                QListWidgetItem *item = list->item(index);
                item->setHidden(!text.isEmpty()
                    && !item->text().contains(text, Qt::CaseInsensitive));
            }

            if (list->count() > 0) {
                for (int index = 0; index < list->count(); ++index) {
                    if (!list->item(index)->isHidden()) {
                        list->setCurrentRow(index);
                        break;
                    }
                }
            }
        });
        connect(filter, &QLineEdit::returnPressed, this, [this]() { run_current(); });
        connect(list, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
            run_current();
        });

        if (list->count() > 0) {
            list->setCurrentRow(0);
        }

        filter->setFocus();
    }

private:
    void run_current() {
        QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString action = item->data(Qt::UserRole).toString();
        accept();

        if (!action.isEmpty()) {
            if (action == QStringLiteral("duplicate_page")
                || action == QStringLiteral("refresh_page")) {
                main->notify(action, main->active_page);
            } else {
                main->notify(action);
            }
        }
    }

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QListWidget *list = nullptr;
};

void show_command_palette(HydrusMainWindow *main) {
    HydrusCommandPalette palette(main);
    palette.exec();
}

// ---------------------------------------------------------------------------
// OR predicate editor
// ---------------------------------------------------------------------------

class HydrusOrDialog : public QDialog {
public:
    HydrusOrDialog(const QString &initial, QWidget *parent = nullptr)
        : QDialog(parent) {
        setWindowTitle(QStringLiteral("OR predicates"));
        resize(520, 380);

        auto *layout = new QVBoxLayout(this);
        auto *help = new QLabel(
            QStringLiteral("One predicate per line. A file matches if any line matches.\n"
                           "Tags, wildcards, namespaces and system: predicates are supported."),
            this);
        help->setWordWrap(true);
        layout->addWidget(help);

        editor = new QPlainTextEdit(this);
        editor->setPlainText(initial);
        editor->setPlaceholderText(QStringLiteral("blue eyes\nspecies:wolf\nsystem:width > 1000"));
        layout->addWidget(editor, 1);

        auto *buttons = new QDialogButtonBox(
            QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
            this);
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

    QString expression() const {
        QStringList parts;

        for (const QString &line : editor->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (!trimmed.isEmpty()) {
                parts.append(trimmed);
            }
        }

        if (parts.isEmpty()) {
            return {};
        }

        return QStringLiteral("{%1}").arg(parts.join(QStringLiteral(" | ")));
    }

private:
    QPlainTextEdit *editor = nullptr;
};

void show_or_predicate_dialog(HydrusMainWindow &main, const QString &page_id, const QString &initial) {
    HydrusOrDialog dialog(initial, &main);

    if (dialog.exec() == QDialog::Accepted) {
        const QString expression = dialog.expression();

        if (!expression.isEmpty()) {
            main.notify(QStringLiteral("or_insert"), page_id, expression);
        }
    }
}

void hydrus_show_or_dialog(HydrusMainWindow &window, rust::Str page_id, rust::Str initial) {
    show_or_predicate_dialog(window, to_qstring(page_id), to_qstring(initial));
}

// ---------------------------------------------------------------------------
// Page manager
// ---------------------------------------------------------------------------

class HydrusPageManager : public QDialog {
public:
    HydrusPageManager(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("manage pages"));
        resize(560, 560);

        auto *layout = new QVBoxLayout(this);

        filter = new QLineEdit(this);
        filter->setPlaceholderText(QStringLiteral("filter pages..."));
        layout->addWidget(filter);

        tree = new QTreeWidget(this);
        tree->setColumnCount(3);
        tree->setHeaderLabels({QStringLiteral("page"), QStringLiteral("type"), QStringLiteral("files")});
        tree->setSelectionMode(QAbstractItemView::ExtendedSelection);
        tree->header()->setSectionResizeMode(0, QHeaderView::Stretch);
        tree->header()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
        tree->header()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
        layout->addWidget(tree, 1);

        auto *buttons = new QHBoxLayout();
        auto *activate = new QPushButton(QStringLiteral("activate"), this);
        auto *close_selected = new QPushButton(QStringLiteral("close selected"), this);
        auto *close_all = new QPushButton(QStringLiteral("close all"), this);
        auto *close_dialog = new QPushButton(QStringLiteral("close"), this);

        buttons->addWidget(activate);
        buttons->addWidget(close_selected);
        buttons->addWidget(close_all);
        buttons->addStretch(1);
        buttons->addWidget(close_dialog);
        layout->addLayout(buttons);

        build(state.value(QStringLiteral("root")).toObject());

        connect(filter, &QLineEdit::textChanged, this, [this](const QString &text) {
            apply_filter(text.trimmed());
        });
        connect(activate, &QPushButton::clicked, this, [this]() { activate_current(); });
        connect(close_selected, &QPushButton::clicked, this, [this]() { close_selected_pages(); });
        connect(close_all, &QPushButton::clicked, this, [this]() { close_all_pages(); });
        connect(close_dialog, &QPushButton::clicked, this, &QDialog::reject);
        connect(tree, &QTreeWidget::itemDoubleClicked, this, [this](QTreeWidgetItem *, int) {
            activate_current();
        });
    }

private:
    QIcon icon_for(const QString &kind) const {
        const QString file_name = hydrus_small_icon_file_name(kind);
        if (!file_name.isEmpty()) {
            const rust::String dir_rust = hydrus_static_dir();
            const QString dir = QString::fromUtf8(dir_rust.data(), static_cast<int>(dir_rust.size()));
            if (!dir.isEmpty()) {
                const QString path = QDir(dir).filePath(file_name);
                if (QFileInfo::exists(path)) {
                    static QHash<QString, QIcon> cache;
                    auto it = cache.find(path);
                    if (it != cache.end()) {
                        return it.value();
                    }
                    QIcon icon(path);
                    if (!icon.isNull()) {
                        cache.insert(path, icon);
                        return icon;
                    }
                }
            }
        }

        if (kind == QStringLiteral("notebook")) {
            return style()->standardIcon(QStyle::SP_DirIcon);
        }

        if (kind == QStringLiteral("query")) {
            return style()->standardIcon(QStyle::SP_FileDialogContentsView);
        }

        if (kind == QStringLiteral("downloader")) {
            return style()->standardIcon(QStyle::SP_ArrowDown);
        }

        if (kind == QStringLiteral("subscriptions")) {
            return style()->standardIcon(QStyle::SP_BrowserReload);
        }

        if (kind == QStringLiteral("duplicates")) {
            return style()->standardIcon(QStyle::SP_FileDialogDetailedView);
        }

        if (kind == QStringLiteral("import_folders")) {
            return style()->standardIcon(QStyle::SP_DirOpenIcon);
        }

        return style()->standardIcon(QStyle::SP_MessageBoxQuestion);
    }

    void build(const QJsonObject &root) {
        tree->clear();
        top_levels.clear();

        const QJsonArray children = root.value(QStringLiteral("children")).toArray();

        for (const QJsonValue &value : children) {
            const QJsonObject child = value.toObject();
            top_levels.append(child.value(QStringLiteral("id")).toString());
            add_node(nullptr, child);
        }

        tree->expandAll();
    }

    QTreeWidgetItem *add_node(QTreeWidgetItem *parent, const QJsonObject &node) {
        const QString id = node.value(QStringLiteral("id")).toString();
        const QString title = node.value(QStringLiteral("title")).toString();
        const QString kind = node.value(QStringLiteral("kind")).toString();
        const int count = node.value(QStringLiteral("count")).toInt();
        const bool active = node.value(QStringLiteral("active")).toBool();

        auto *item = parent == nullptr ? new QTreeWidgetItem(tree) : new QTreeWidgetItem(parent);
        item->setText(0, title.isEmpty() ? QStringLiteral("page") : title);
        item->setText(1, kind);
        item->setText(2, QString::number(count));
        item->setIcon(0, icon_for(kind));
        item->setData(0, Qt::UserRole, id);

        if (active) {
            QFont font = item->font(0);
            font.setBold(true);
            item->setFont(0, font);
        }

        const QJsonArray children = node.value(QStringLiteral("children")).toArray();

        for (const QJsonValue &value : children) {
            add_node(item, value.toObject());
        }

        return item;
    }

    bool filter_item(QTreeWidgetItem *item, const QString &needle) {
        bool child_visible = false;

        for (int index = 0; index < item->childCount(); ++index) {
            child_visible = filter_item(item->child(index), needle) || child_visible;
        }

        const bool matches = needle.isEmpty()
            || item->text(0).contains(needle, Qt::CaseInsensitive)
            || item->text(1).contains(needle, Qt::CaseInsensitive);
        const bool visible = matches || child_visible;

        item->setHidden(!visible);

        return visible;
    }

    void apply_filter(const QString &needle) {
        for (int index = 0; index < tree->topLevelItemCount(); ++index) {
            filter_item(tree->topLevelItem(index), needle);
        }
    }

    QString id_of(QTreeWidgetItem *item) const {
        return item == nullptr ? QString() : item->data(0, Qt::UserRole).toString();
    }

    void activate_current() {
        const QString id = id_of(tree->currentItem());

        if (id.isEmpty()) {
            return;
        }

        main->notify(QStringLiteral("page_manager_activate"), id);
        accept();
    }

    void close_selected_pages() {
        QJsonArray ids;

        for (QTreeWidgetItem *item : tree->selectedItems()) {
            bool ancestor_selected = false;

            for (QTreeWidgetItem *parent = item->parent(); parent != nullptr; parent = parent->parent()) {
                if (parent->isSelected()) {
                    ancestor_selected = true;
                    break;
                }
            }

            if (!ancestor_selected) {
                ids.append(id_of(item));
            }
        }

        if (ids.isEmpty()) {
            return;
        }

        const QByteArray payload = QJsonDocument(ids).toJson(QJsonDocument::Compact);
        main->notify(QStringLiteral("page_manager_close"), QString(), QString::fromUtf8(payload));
        accept();
    }

    void close_all_pages() {
        QJsonArray ids;

        for (const QString &id : top_levels) {
            ids.append(id);
        }

        if (ids.isEmpty()) {
            return;
        }

        const QByteArray payload = QJsonDocument(ids).toJson(QJsonDocument::Compact);
        main->notify(QStringLiteral("page_manager_close"), QString(), QString::fromUtf8(payload));
        accept();
    }

    HydrusMainWindow *main = nullptr;
    QLineEdit *filter = nullptr;
    QTreeWidget *tree = nullptr;
    QStringList top_levels;
};

void HydrusMainWindow::show_page_chooser(const QString &notebook_id) {
    HydrusPageChooser chooser(notebook_id, this);

    if (chooser.exec() == QDialog::Accepted && !chooser.result.isEmpty()) {
        notify(QStringLiteral("create_page"), notebook_id, chooser.result);
    }
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

class HydrusManageTagsDialog : public QDialog {
public:
    HydrusManageTagsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window), data(state) {
        setWindowTitle(QStringLiteral("manage tags"));
        resize(460, 600);

        auto *layout = new QVBoxLayout(this);

        services = new QComboBox(this);
        layout->addWidget(services);

        summary = new QLabel(this);
        layout->addWidget(summary);

        tags = new QListWidget(this);
        tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
        layout->addWidget(tags, 1);

        auto *input_row = new QHBoxLayout();
        input = new QLineEdit(this);
        input->setPlaceholderText(QStringLiteral("tag1, tag2, namespace:tag"));
        input_row->addWidget(input, 1);

        auto *add = new QPushButton(QStringLiteral("add"), this);
        auto *remove = new QPushButton(QStringLiteral("remove"), this);
        input_row->addWidget(add);
        input_row->addWidget(remove);
        layout->addLayout(input_row);

        auto *close_box = new QDialogButtonBox(QDialogButtonBox::Close, this);
        connect(close_box, &QDialogButtonBox::rejected, this, &QDialog::accept);
        layout->addWidget(close_box);

        for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
            const QString name = value.toObject().value(QStringLiteral("name")).toString();

            if (!name.isEmpty()) {
                services->addItem(name);
            }
        }

        summary->setText(QStringLiteral("%1 file(s) selected")
            .arg(data.value(QStringLiteral("hashes")).toArray().size()));

        connect(services, &QComboBox::currentTextChanged, this, [this](const QString &) {
            rebuild();
        });
        connect(add, &QPushButton::clicked, this, [this]() { add_tags(); });
        connect(remove, &QPushButton::clicked, this, [this]() { remove_tags(); });
        connect(input, &QLineEdit::returnPressed, this, [this]() { add_tags(); });

        rebuild();
    }

private:
    QJsonObject current_service() const {
        const QString name = services->currentText();

        for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
            const QJsonObject service = value.toObject();

            if (service.value(QStringLiteral("name")).toString() == name) {
                return service;
            }
        }

        return {};
    }

    void rebuild() {
        tags->clear();

        for (const QJsonValue &value : current_service().value(QStringLiteral("tags")).toArray()) {
            const QJsonObject tag = value.toObject();
            const QString name = tag.value(QStringLiteral("tag")).toString();
            const int count = tag.value(QStringLiteral("count")).toInt();
            auto *item = new QListWidgetItem(
                count > 1 ? QStringLiteral("%1 (%2)").arg(name).arg(count) : name,
                tags);
            item->setData(Qt::UserRole, name);
        }
    }

    void send(const QStringList &values, const QString &operation) {
        if (values.isEmpty() || services->currentText().isEmpty()) {
            return;
        }

        QJsonArray hashes = data.value(QStringLiteral("hashes")).toArray();
        QJsonArray array;

        for (const QString &value : values) {
            array.append(value);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("hashes"), hashes);
        payload.insert(QStringLiteral("service"), services->currentText());
        payload.insert(operation, array);
        main->notify(
            QStringLiteral("apply_tags"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
    }

    void add_tags() {
        QStringList values;

        for (const QString &part : input->text().split(QRegularExpression(QStringLiteral("[,;\\n]")))) {
            const QString tag = part.trimmed();

            if (!tag.isEmpty()) {
                values.append(tag);
            }
        }

        if (values.isEmpty()) {
            return;
        }

        send(values, QStringLiteral("add"));

        for (const QString &tag : values) {
            bool exists = false;

            for (int index = 0; index < tags->count(); ++index) {
                if (tags->item(index)->data(Qt::UserRole).toString() == tag) {
                    exists = true;
                    break;
                }
            }

            if (!exists) {
                auto *item = new QListWidgetItem(tag, tags);
                item->setData(Qt::UserRole, tag);
            }
        }

        input->clear();
    }

    void remove_tags() {
        QStringList values;

        for (QListWidgetItem *item : tags->selectedItems()) {
            values.append(item->data(Qt::UserRole).toString());
        }

        if (values.isEmpty()) {
            return;
        }

        send(values, QStringLiteral("remove"));

        for (QListWidgetItem *item : tags->selectedItems()) {
            delete item;
        }
    }

    HydrusMainWindow *main = nullptr;
    QJsonObject data;
    QComboBox *services = nullptr;
    QLabel *summary = nullptr;
    QListWidget *tags = nullptr;
    QLineEdit *input = nullptr;
};

class HydrusTimestampsDialog : public QDialog {
public:
    HydrusTimestampsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window), hash(state.value(QStringLiteral("hash")).toString()) {
        setWindowTitle(QStringLiteral("edit timestamps"));
        resize(420, 220);

        auto *layout = new QVBoxLayout(this);
        auto *form = new QFormLayout();

        archived_enabled = new QCheckBox(QStringLiteral("archived time"), this);
        archived = new QDateTimeEdit(this);
        archived->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        archived->setCalendarPopup(true);
        form->addRow(archived_enabled, archived);

        modified_enabled = new QCheckBox(QStringLiteral("file modified time"), this);
        modified = new QDateTimeEdit(this);
        modified->setDisplayFormat(QStringLiteral("yyyy-MM-dd HH:mm:ss"));
        modified->setCalendarPopup(true);
        form->addRow(modified_enabled, modified);

        layout->addLayout(form);

        set_time(archived_enabled, archived, state.value(QStringLiteral("archived")));
        set_time(modified_enabled, modified, state.value(QStringLiteral("modified")));

        auto *buttons = new QDialogButtonBox(
            QDialogButtonBox::Save | QDialogButtonBox::Cancel,
            this);
        connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
    }

private:
    void set_time(QCheckBox *enabled, QDateTimeEdit *editor, const QJsonValue &value) {
        if (value.isDouble()) {
            enabled->setChecked(true);
            editor->setDateTime(QDateTime::fromSecsSinceEpoch(value.toInteger()));
        } else {
            enabled->setChecked(false);
            editor->setDateTime(QDateTime::currentDateTime());
        }
    }

    void save() {
        QJsonObject payload;
        payload.insert(QStringLiteral("hash"), hash);
        payload.insert(
            QStringLiteral("archived"),
            archived_enabled->isChecked()
                ? QJsonValue(archived->dateTime().toSecsSinceEpoch())
                : QJsonValue(QJsonValue::Null));
        payload.insert(
            QStringLiteral("modified"),
            modified_enabled->isChecked()
                ? QJsonValue(modified->dateTime().toSecsSinceEpoch())
                : QJsonValue(QJsonValue::Null));

        main->notify(
            QStringLiteral("timestamps_save"),
            QString(),
            QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
        accept();
    }

    HydrusMainWindow *main = nullptr;
    QString hash;
    QCheckBox *archived_enabled = nullptr;
    QCheckBox *modified_enabled = nullptr;
    QDateTimeEdit *archived = nullptr;
    QDateTimeEdit *modified = nullptr;
};

// ---------------------------------------------------------------------------
// Popup messages
// ---------------------------------------------------------------------------

class HydrusNotificationsDialog : public QDialog {
public:
    HydrusNotificationsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent = nullptr)
        : QDialog(parent), main(main_window) {
        setWindowTitle(QStringLiteral("popup messages"));
        resize(720, 460);

        auto *layout = new QVBoxLayout(this);

        summary = new QLabel(
            QStringLiteral("%1 message(s)").arg(state.value(QStringLiteral("count")).toInt()),
            this);
        layout->addWidget(summary);

        table = new QTableWidget(this);
        table->setColumnCount(3);
        table->setHorizontalHeaderLabels({
            QStringLiteral("time"),
            QStringLiteral("level"),
            QStringLiteral("message"),
        });
        table->verticalHeader()->setVisible(false);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
        table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
        table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
        layout->addWidget(table, 1);

        rebuild(state.value(QStringLiteral("entries")).toArray());

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
        auto *clear = new QPushButton(QStringLiteral("clear"), this);
        buttons->addButton(clear, QDialogButtonBox::ActionRole);
        connect(clear, &QPushButton::clicked, this, [this]() {
            table->setRowCount(0);
            summary->setText(QStringLiteral("0 message(s)"));
            main->notify(QStringLiteral("notifications_clear"));
        });
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::accept);
        layout->addWidget(buttons);
    }

private:
    void rebuild(const QJsonArray &entries) {
        table->setRowCount(entries.size());

        for (int row = 0; row < entries.size(); ++row) {
            const QJsonObject entry = entries.at(row).toObject();

            table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("time")).toString()));
            table->setItem(row, 1, new QTableWidgetItem(entry.value(QStringLiteral("level")).toString()));
            table->setItem(row, 2, new QTableWidgetItem(entry.value(QStringLiteral("text")).toString()));
        }
    }

    HydrusMainWindow *main = nullptr;
    QLabel *summary = nullptr;
    QTableWidget *table = nullptr;
};

void HydrusMainWindow::show_viewing_stats(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("file viewing statistics"));
    dialog.resize(560, 260);

    auto *layout = new QVBoxLayout(&dialog);
    auto *hash = new QLabel(
        QStringLiteral("file: %1").arg(state.value(QStringLiteral("hash")).toString()),
        &dialog);
    hash->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(hash);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("canvas"),
        QStringLiteral("views"),
        QStringLiteral("viewtime (s)"),
        QStringLiteral("last viewed"),
    });
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);

    const QJsonArray rows = state.value(QStringLiteral("rows")).toArray();
    table->setRowCount(rows.size());

    for (int row = 0; row < rows.size(); ++row) {
        const QJsonObject entry = rows.at(row).toObject();
        table->setItem(row, 0, new QTableWidgetItem(entry.value(QStringLiteral("canvas")).toString()));
        table->setItem(row, 1, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("views")).toInt())));
        table->setItem(row, 2, new QTableWidgetItem(QString::number(entry.value(QStringLiteral("viewtime")).toDouble())));
        table->setItem(row, 3, new QTableWidgetItem(entry.value(QStringLiteral("last_viewed")).toString()));
    }

    layout->addWidget(table, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_about(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("about"));
    dialog.resize(680, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *title = new QLabel(QStringLiteral("Hydrus Network"), &dialog);
    title->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold;"));
    layout->addWidget(title);

    auto *subtitle = new QLabel(
        QStringLiteral("Rust/Qt client · software v%1 · network v%2")
            .arg(state.value(QStringLiteral("software_version")).toInt())
            .arg(state.value(QStringLiteral("network_version")).toInt()),
        &dialog);
    layout->addWidget(subtitle);

    auto *tabs = new QTabWidget(&dialog);
    layout->addWidget(tabs, 1);

    auto *overview = new QWidget(tabs);
    auto *form = new QFormLayout(overview);
    const auto add_row = [form](const QString &label, const QString &value) {
        auto *field = new QLabel(value, form->parentWidget());
        field->setTextInteractionFlags(Qt::TextSelectableByMouse);
        field->setWordWrap(true);
        form->addRow(label, field);
    };

    add_row(QStringLiteral("software version"), QString::number(state.value(QStringLiteral("software_version")).toInt()));
    add_row(QStringLiteral("network version"), QString::number(state.value(QStringLiteral("network_version")).toInt()));
    add_row(QStringLiteral("client api version"), QString::number(state.value(QStringLiteral("client_api_version")).toInt()));
    add_row(QStringLiteral("db version"), QString::number(state.value(QStringLiteral("db_version")).toInt()));
    add_row(QStringLiteral("client api"), state.value(QStringLiteral("api")).toString());
    add_row(QStringLiteral("sqlite"), state.value(QStringLiteral("sqlite")).toString());
    add_row(QStringLiteral("rust crate"), state.value(QStringLiteral("rust")).toString());
    add_row(QStringLiteral("Qt"), QString::fromLatin1(qVersion()));
    add_row(QStringLiteral("services"), QString::number(state.value(QStringLiteral("services")).toInt()));
    add_row(QStringLiteral("file locations"), QString::number(state.value(QStringLiteral("locations")).toInt()));
    add_row(QStringLiteral("shortcuts"), QString::number(state.value(QStringLiteral("shortcuts")).toInt()));
    add_row(QStringLiteral("db dir"), state.value(QStringLiteral("db_dir")).toString());
    add_row(QStringLiteral("install dir"), state.value(QStringLiteral("install_dir")).toString());
    tabs->addTab(overview, QStringLiteral("overview"));

    auto *license = new QPlainTextEdit(tabs);
    license->setReadOnly(true);
    license->setPlainText(
        state.value(QStringLiteral("license")).toString() + QStringLiteral("\n\n")
        + state.value(QStringLiteral("site")).toString());
    tabs->addTab(license, QStringLiteral("license"));

    auto *credits = new QPlainTextEdit(tabs);
    credits->setReadOnly(true);
    credits->setPlainText(state.value(QStringLiteral("credits")).toString());
    tabs->addTab(credits, QStringLiteral("credits"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *site = new QPushButton(QStringLiteral("open site"), &dialog);
    buttons->addButton(site, QDialogButtonBox::ActionRole);
    connect(site, &QPushButton::clicked, &dialog, [&state]() {
        QDesktopServices::openUrl(QUrl(state.value(QStringLiteral("site")).toString()));
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

void HydrusMainWindow::show_notifications(const QJsonObject &state) {
    HydrusNotificationsDialog dialog(this, state, this);
    dialog.exec();
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

void HydrusMainWindow::show_notes(const QJsonObject &state) {
    const QString hash = state.value(QStringLiteral("hash")).toString();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("notes"));
    dialog.resize(620, 460);

    auto *layout = new QVBoxLayout(&dialog);
    auto *info = new QLabel(hash.left(16) + QStringLiteral("..."), &dialog);
    info->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(info);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(2);
    table->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("note")});
    table->horizontalHeader()->setStretchLastSection(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);

    const QJsonArray notes = state.value(QStringLiteral("notes")).toArray();
    table->setRowCount(notes.size());

    for (int index = 0; index < notes.size(); ++index) {
        const QJsonObject note = notes.at(index).toObject();
        table->setItem(index, 0, new QTableWidgetItem(note.value(QStringLiteral("name")).toString()));
        table->setItem(index, 1, new QTableWidgetItem(note.value(QStringLiteral("text")).toString()));
    }

    table->resizeRowsToContents();
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add note"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons->addWidget(add);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    auto *box = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, &dialog);
    buttons->addWidget(box);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->rowCount();
        table->insertRow(row);
        table->setItem(row, 0, new QTableWidgetItem());
        table->setItem(row, 1, new QTableWidgetItem());
        table->editItem(table->item(row, 0));
    });
    connect(remove, &QPushButton::clicked, &dialog, [table]() {
        const int row = table->currentRow();

        if (row >= 0) {
            table->removeRow(row);
        }
    });
    connect(box, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(box, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray values;

    for (int row = 0; row < table->rowCount(); ++row) {
        const QString name = table->item(row, 0) == nullptr
            ? QString()
            : table->item(row, 0)->text().trimmed();
        const QString text = table->item(row, 1) == nullptr
            ? QString()
            : table->item(row, 1)->text();

        if (name.isEmpty() && text.trimmed().isEmpty()) {
            continue;
        }

        QJsonObject note;
        note.insert(QStringLiteral("name"), name);
        note.insert(QStringLiteral("text"), text);
        values.append(note);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("notes"), values);

    notify(QStringLiteral("notes_save"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_tag_relations(const QJsonObject &state) {
    if (tag_relations_dialog == nullptr) {
        tag_relations_dialog = new HydrusTagRelationsDialog(this, state, this);
        connect(tag_relations_dialog, &QObject::destroyed, this, [this]() {
            tag_relations_dialog = nullptr;
        });
        tag_relations_dialog->show();
    } else {
        tag_relations_dialog->set_state(state);
    }

    tag_relations_dialog->raise();
    tag_relations_dialog->activateWindow();
}

HydrusLocalImportReviewDialog::HydrusLocalImportReviewDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("review files to import"));
    setObjectName(QStringLiteral("hydrus_local_import_review"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(960, 560);

    auto *layout = new QVBoxLayout(this);

    summary = new QLabel(this);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    auto *controls = new QHBoxLayout();
    auto *add_files = new QPushButton(QStringLiteral("add files"), this);
    auto *add_folder = new QPushButton(QStringLiteral("add folder"), this);
    auto *remove = new QPushButton(QStringLiteral("remove files"), this);
    search = new QCheckBox(QStringLiteral("search subdirectories"), this);
    controls->addWidget(add_files);
    controls->addWidget(add_folder);
    controls->addWidget(remove);
    controls->addWidget(search);
    controls->addStretch(1);
    layout->addLayout(controls);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("local_import_paths"));
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("#"), QStringLiteral("path"),
                                      QStringLiteral("result / mime"), QStringLiteral("size")});
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSortingEnabled(true);
    layout->addWidget(table, 1);

    auto *delete_row = new QHBoxLayout();
    warning = new QLabel(this);
    warning->setObjectName(QStringLiteral("HydrusWarning"));
    delete_after_success = new QCheckBox(QStringLiteral("delete original files after successful import"), this);
    delete_row->addWidget(warning, 1);
    delete_row->addWidget(delete_after_success);
    layout->addLayout(delete_row);

    auto *buttons = new QHBoxLayout();
    auto *options = new QPushButton(QStringLiteral("import options..."), this);
    tag_button = new QPushButton(QStringLiteral("add tags/urls with the import >>"), this);
    tag_button->setToolTip(QStringLiteral(
        "Add tags from filenames to local tag services, then start the import."));
    import_button = new QPushButton(QStringLiteral("import now"), this);
    import_button->setObjectName(QStringLiteral("HydrusAccept"));
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(options);
    buttons->addStretch(1);
    buttons->addWidget(tag_button);
    buttons->addWidget(import_button);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(add_files, &QPushButton::clicked, this, [this]() {
        const QStringList paths = QFileDialog::getOpenFileNames(
            this,
            QStringLiteral("import files"),
            QString(),
            QStringLiteral("all files (*.*)"));

        if (paths.isEmpty()) {
            return;
        }

        QJsonObject payload;
        QJsonArray array;
        for (const QString &path : paths) {
            array.append(path);
        }
        payload.insert(QStringLiteral("paths"), array);
        send(QStringLiteral("local_import_add_paths"), payload);
    });
    connect(add_folder, &QPushButton::clicked, this, [this]() {
        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select a folder to add."));

        if (path.isEmpty()) {
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("paths"), QJsonArray{path});
        send(QStringLiteral("local_import_add_paths"), payload);
    });
    connect(remove, &QPushButton::clicked, this, [this]() {
        const QStringList paths = selected_paths();

        if (paths.isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select some paths first."));
            return;
        }

        QJsonObject payload;
        QJsonArray array;
        for (const QString &path : paths) {
            array.append(path);
        }
        payload.insert(QStringLiteral("paths"), array);
        send(QStringLiteral("local_import_remove"), payload);
    });
    connect(search, &QCheckBox::toggled, this, [this](bool checked) {
        QJsonObject payload;
        payload.insert(QStringLiteral("value"), checked);
        send(QStringLiteral("local_import_search_subdirectories"), payload);
    });
    connect(delete_after_success, &QCheckBox::toggled, this, [this](bool checked) {
        warning->setText(checked ? QStringLiteral("YOUR ORIGINAL FILES WILL BE DELETED")
                                 : QString());
        QJsonObject payload;
        payload.insert(QStringLiteral("value"), checked);
        send(QStringLiteral("local_import_delete_after_success"), payload);
    });
    connect(options, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_options"));
    });
    connect(tag_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_tagging_open"));
    });
    connect(import_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("local_import_import"));
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

QStringList HydrusLocalImportReviewDialog::selected_paths() const {
    QStringList paths;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString path = item->data(Qt::UserRole).toString();

        if (!path.isEmpty() && !paths.contains(path)) {
            paths << path;
        }
    }

    return paths;
}

void HydrusLocalImportReviewDialog::send(const QString &action, const QJsonObject &payload) {
    main->notify(action, QString(), payload.isEmpty() ? QString() : json_payload(payload));
}

void HydrusLocalImportReviewDialog::set_state(const QJsonObject &state) {
    data = state;

    summary->setText(state.value(QStringLiteral("summary")).toString()
                     + (state.value(QStringLiteral("note")).toString().isEmpty()
                            ? QString()
                            : QStringLiteral("\n%1").arg(state.value(QStringLiteral("note")).toString())));

    {
        QSignalBlocker blocker(search);
        search->setChecked(state.value(QStringLiteral("search_subdirectories")).toBool(true));
    }
    {
        QSignalBlocker blocker(delete_after_success);
        const bool checked = state.value(QStringLiteral("delete_after_success")).toBool(false);
        delete_after_success->setChecked(checked);
        warning->setText(checked ? QStringLiteral("YOUR ORIGINAL FILES WILL BE DELETED")
                                 : QString());
    }

    const QStringList selected = selected_paths();

    table->setSortingEnabled(false);
    table->setRowCount(0);

    for (const QJsonValue &value : state.value(QStringLiteral("paths")).toArray()) {
        const QJsonObject parse = value.toObject();
        const QString path = parse.value(QStringLiteral("path")).toString();
        const int row = table->rowCount();
        table->insertRow(row);

        table->setItem(row, 0, new QTableWidgetItem(QString::number(parse.value(QStringLiteral("index")).toInt())));

        auto *path_item = new QTableWidgetItem(path);
        path_item->setData(Qt::UserRole, path);
        path_item->setToolTip(path);
        table->setItem(row, 1, path_item);

        auto *result_item = new QTableWidgetItem(parse.value(QStringLiteral("result_text")).toString());

        if (parse.value(QStringLiteral("bad")).toBool()) {
            result_item->setForeground(QColor(220, 110, 110));
        } else if (parse.value(QStringLiteral("sidecar")).toBool()) {
            result_item->setForeground(QColor(170, 170, 170));
        } else {
            result_item->setForeground(QColor(120, 190, 120));
        }

        table->setItem(row, 2, result_item);
        table->setItem(row, 3, new QTableWidgetItem(parse.value(QStringLiteral("size_display")).toString()));
    }

    table->setSortingEnabled(true);

    for (int row = 0; row < table->rowCount(); ++row) {
        if (selected.contains(table->item(row, 1)->data(Qt::UserRole).toString())) {
            table->selectRow(row);
        }
    }

    const bool can_import = state.value(QStringLiteral("can_import")).toBool(false);
    import_button->setEnabled(can_import);
    tag_button->setEnabled(can_import);
    import_button->setText(can_import
        ? QStringLiteral("import now (%1)").arg(state.value(QStringLiteral("good_count")).toInt())
        : QStringLiteral("import now"));
}

HydrusLocalImportTaggingDialog::HydrusLocalImportTaggingDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("filename tagging"));
    setObjectName(QStringLiteral("hydrus_local_import_tagging"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(640, 520);

    auto *layout = new QVBoxLayout(this);

    help = new QLabel(this);
    help->setWordWrap(true);
    layout->addWidget(help);

    list = new QListWidget(this);
    list->setObjectName(QStringLiteral("local_import_filename_tagging"));
    layout->addWidget(list, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add tag service..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    auto *footer = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *save = new QPushButton(QStringLiteral("save and import"), this);
    save->setObjectName(QStringLiteral("HydrusAccept"));
    footer->addStretch(1);
    footer->addWidget(cancel);
    footer->addWidget(save);
    layout->addLayout(footer);

    connect(add, &QPushButton::clicked, this, [this]() {
        QStringList available;

        for (auto iterator = service_name_to_key.constBegin(); iterator != service_name_to_key.constEnd(); ++iterator) {
            if (!filename_tagging.contains(iterator.value())) {
                available << iterator.key();
            }
        }

        if (available.isEmpty()) {
            QMessageBox::information(
                this,
                windowTitle(),
                QStringLiteral("Create a local tag service first, or every local tag service already has rules."));
            return;
        }

        available.sort();
        bool ok = false;
        const QString name = QInputDialog::getItem(
            this,
            windowTitle(),
            QStringLiteral("tag service:"),
            available,
            0,
            false,
            &ok);

        if (!ok || name.isEmpty()) {
            return;
        }

        edit_rules(service_name_to_key.value(name), name, QJsonObject());
    });
    connect(edit, &QPushButton::clicked, this, [this]() {
        const QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();
        edit_rules(key, service_name_to_key.key(key, key), filename_tagging.value(key).toObject());
    });
    connect(remove, &QPushButton::clicked, this, [this]() {
        const QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();

        if (QMessageBox::question(
                this,
                windowTitle(),
                QStringLiteral("Remove the filename tagging rules for this service?"))
            != QMessageBox::Yes) {
            return;
        }

        filename_tagging.remove(key);
        rebuild();
    });
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(save, &QPushButton::clicked, this, [this]() { this->save(); });

    set_state(state);
}

void HydrusLocalImportTaggingDialog::rebuild() {
    list->clear();

    for (auto iterator = filename_tagging.constBegin(); iterator != filename_tagging.constEnd(); ++iterator) {
        const QString name = service_name_to_key.key(iterator.key(), iterator.key());
        auto *item = new QListWidgetItem(
            QStringLiteral("%1 - %2").arg(name, filename_tagging_summary(iterator.value().toObject())),
            list);
        item->setData(Qt::UserRole, iterator.key());
    }
}

void HydrusLocalImportTaggingDialog::edit_rules(
    const QString &key, const QString &name, const QJsonObject &current) {
    QJsonObject initial = current.isEmpty() ? default_filename_tagging_options() : current;

    if (!example_path.isEmpty()) {
        initial.insert(QStringLiteral("example_path"), example_path);
    }

    QJsonObject edited;

    if (!edit_filename_tagging_options(this, name, initial, &edited)) {
        return;
    }

    if (filename_tagging_summary(edited) == QStringLiteral("no tags")) {
        filename_tagging.remove(key);
    } else {
        filename_tagging.insert(key, edited);
    }

    rebuild();
}

void HydrusLocalImportTaggingDialog::save() {
    QJsonObject payload;
    payload.insert(QStringLiteral("filename_tagging"), filename_tagging);
    payload.insert(QStringLiteral("import_after_save"), import_after_save);
    main->notify(QStringLiteral("local_import_tagging_save"), QString(), json_payload(payload));
    close();
}

void HydrusLocalImportTaggingDialog::set_state(const QJsonObject &state) {
    data = state;
    filename_tagging = state.value(QStringLiteral("filename_tagging")).toObject();
    example_path = state.value(QStringLiteral("example_path")).toString();
    import_after_save = state.value(QStringLiteral("import_after_save")).toBool(false);
    service_name_to_key.clear();

    for (const QJsonValue &value : state.value(QStringLiteral("tag_services")).toArray()) {
        const QJsonObject service = value.toObject();
        service_name_to_key.insert(
            service.value(QStringLiteral("name")).toString(),
            service.value(QStringLiteral("key")).toString());
    }

    help->setText(import_after_save
        ? QStringLiteral("A tag service can read tags from each file's path. When you save, "
                         "the import starts with these rules and your existing import options.")
        : QStringLiteral("A tag service can read tags from each file's path."));
    rebuild();
}

HydrusRepairFileLocationsDialog::HydrusRepairFileLocationsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("repair file locations"));
    setObjectName(QStringLiteral("hydrus_repair_file_locations"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 540);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Some expected file storage directories were not found. Set the correct "
                       "base folder for each prefix, or probe a folder that contains the old "
                       "client_files/thumbnails layout. File prefixes must be corrected; "
                       "thumbnail prefixes can be recreated empty."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    summary = new QLabel(this);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    table = new QTableWidget(this);
    table->setObjectName(QStringLiteral("repair_file_locations"));
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({QStringLiteral("current base"), QStringLiteral("prefix"),
                                      QStringLiteral("corrected base"), QStringLiteral("status")});
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->verticalHeader()->setVisible(false);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    layout->addWidget(table, 1);

    message = new QLabel(this);
    message->setWordWrap(true);
    layout->addWidget(message);

    auto *controls = new QHBoxLayout();
    create_thumbnails = new QCheckBox(
        QStringLiteral("create empty folders for missing thumbnail paths"), this);
    create_thumbnails->setChecked(true);
    auto *set_button = new QPushButton(QStringLiteral("set correct location"), this);
    auto *probe_button = new QPushButton(
        QStringLiteral("add a possibly correct location (let the client figure out what it contains)"),
        this);
    auto *apply_button = new QPushButton(QStringLiteral("apply"), this);
    apply_button->setObjectName(QStringLiteral("HydrusAccept"));
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    controls->addWidget(create_thumbnails);
    controls->addStretch(1);
    controls->addWidget(set_button);
    controls->addWidget(probe_button);
    controls->addWidget(apply_button);
    controls->addWidget(close_button);
    layout->addLayout(controls);

    connect(set_button, &QPushButton::clicked, this, [this]() {
        const QStringList prefixes = selected_prefixes();

        if (prefixes.isEmpty()) {
            QMessageBox::information(this, windowTitle(), QStringLiteral("Select one or more rows first."));
            return;
        }

        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select correct location."));

        if (path.isEmpty()) {
            return;
        }

        QJsonArray updates;
        for (const QString &prefix : prefixes) {
            QJsonObject update;
            update.insert(QStringLiteral("prefix"), prefix);
            update.insert(QStringLiteral("base"), path);
            updates.append(update);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("updates"), updates);
        send(QStringLiteral("repair_locations_set"), payload);
    });
    connect(probe_button, &QPushButton::clicked, this, [this]() {
        const QString path = QFileDialog::getExistingDirectory(
            this,
            QStringLiteral("Select the potential correct location."));

        if (path.isEmpty()) {
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("path"), path);
        send(QStringLiteral("repair_locations_probe"), payload);
    });
    connect(apply_button, &QPushButton::clicked, this, [this]() {
        QJsonObject payload;
        payload.insert(QStringLiteral("create_thumbnails"), create_thumbnails->isChecked());
        send(QStringLiteral("repair_locations_apply"), payload);
    });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

QStringList HydrusRepairFileLocationsDialog::selected_prefixes() const {
    QStringList prefixes;

    for (const QTableWidgetItem *item : table->selectedItems()) {
        if (item->column() != 1) {
            continue;
        }

        const QString prefix = item->data(Qt::UserRole).toString();

        if (!prefix.isEmpty() && !prefixes.contains(prefix)) {
            prefixes << prefix;
        }
    }

    return prefixes;
}

void HydrusRepairFileLocationsDialog::send(const QString &action, const QJsonObject &payload) {
    main->notify(action, QString(), payload.isEmpty() ? QString() : json_payload(payload));
}

void HydrusRepairFileLocationsDialog::set_state(const QJsonObject &state) {
    data = state;
    summary->setText(state.value(QStringLiteral("summary")).toString());
    message->setText(state.value(QStringLiteral("message")).toString());

    const QString error = state.value(QStringLiteral("error")).toString();

    if (!error.isEmpty()) {
        QMessageBox::warning(this, windowTitle(), error);
    }

    table->setRowCount(0);

    for (const QJsonValue &value : state.value(QStringLiteral("rows")).toArray()) {
        const QJsonObject row = value.toObject();
        const int index = table->rowCount();
        table->insertRow(index);

        table->setItem(index, 0, new QTableWidgetItem(row.value(QStringLiteral("old_base")).toString()));
        table->setItem(index, 1, new QTableWidgetItem(row.value(QStringLiteral("prefix")).toString()));
        table->item(index, 1)->setData(Qt::UserRole, row.value(QStringLiteral("prefix")).toString());
        table->setItem(index, 2, new QTableWidgetItem(row.value(QStringLiteral("base")).toString()));
        auto *status = new QTableWidgetItem(row.value(QStringLiteral("status")).toString());

        if (row.value(QStringLiteral("exists")).toBool()) {
            status->setForeground(QColor(120, 190, 120));
        } else {
            status->setForeground(QColor(220, 110, 110));
        }

        table->setItem(index, 3, status);
    }
}

void HydrusMainWindow::show_local_import_review(const QJsonObject &state) {
    if (local_import_review_dialog == nullptr) {
        local_import_review_dialog = new HydrusLocalImportReviewDialog(this, state, this);
        connect(local_import_review_dialog, &QObject::destroyed, this, [this]() {
            local_import_review_dialog = nullptr;
        });
        local_import_review_dialog->show();
    } else {
        local_import_review_dialog->set_state(state);
        local_import_review_dialog->raise();
        local_import_review_dialog->activateWindow();
    }
}

void HydrusMainWindow::show_local_import_tagging(const QJsonObject &state) {
    if (local_import_tagging_dialog == nullptr) {
        local_import_tagging_dialog = new HydrusLocalImportTaggingDialog(this, state, this);
        connect(local_import_tagging_dialog, &QObject::destroyed, this, [this]() {
            local_import_tagging_dialog = nullptr;
        });
        local_import_tagging_dialog->show();
    } else {
        local_import_tagging_dialog->set_state(state);
        local_import_tagging_dialog->raise();
        local_import_tagging_dialog->activateWindow();
    }
}

void HydrusMainWindow::close_local_import_review() {
    if (local_import_tagging_dialog != nullptr) {
        local_import_tagging_dialog->close();
    }

    if (local_import_review_dialog != nullptr) {
        local_import_review_dialog->close();
    }
}

void HydrusMainWindow::show_repair_file_locations(const QJsonObject &state) {
    if (repair_file_locations_dialog == nullptr) {
        repair_file_locations_dialog = new HydrusRepairFileLocationsDialog(this, state, this);
        connect(repair_file_locations_dialog, &QObject::destroyed, this, [this]() {
            repair_file_locations_dialog = nullptr;
        });
        repair_file_locations_dialog->show();
    } else {
        repair_file_locations_dialog->set_state(state);
        repair_file_locations_dialog->raise();
        repair_file_locations_dialog->activateWindow();
    }
}

void HydrusMainWindow::close_repair_file_locations() {
    if (repair_file_locations_dialog != nullptr) {
        repair_file_locations_dialog->close();
    }
}

void HydrusMainWindow::show_export_review(const QJsonObject &state) {
    const QString name = state.value(QStringLiteral("name")).toString();
    const QJsonArray files = state.value(QStringLiteral("files")).toArray();

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("review export folder '%1'").arg(name));
    dialog.resize(860, 520);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("%1 -> %2\n%3 files match the search. Uncheck anything you do not want "
                       "to export this run.")
            .arg(name, state.value(QStringLiteral("destination")).toString())
            .arg(files.size()),
        &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    auto *table = new QTableWidget(&dialog);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("export"),
        QStringLiteral("file"),
        QStringLiteral("tags"),
        QStringLiteral("destination"),
        QStringLiteral("state")
    });
    table->horizontalHeader()->setStretchLastSection(false);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setTextElideMode(Qt::ElideMiddle);
    table->verticalHeader()->setVisible(false);
    table->setRowCount(files.size());

    for (int index = 0; index < files.size(); ++index) {
        const QJsonObject file = files.at(index).toObject();
        const QString hash = file.value(QStringLiteral("hash")).toString();
        const bool safe = file.value(QStringLiteral("safe")).toBool();
        const bool source_exists = file.value(QStringLiteral("source_exists")).toBool(true);
        const bool exists = file.value(QStringLiteral("exists")).toBool();
        const QString destination = file.value(QStringLiteral("destination")).toString();
        const QString root_dir = state.value(QStringLiteral("destination")).toString();
        const QString relative = destination.startsWith(root_dir)
            ? destination.mid(root_dir.size()).replace(QLatin1Char('\\'), QLatin1Char('/'))
            : destination;

        for (int column = 0; column < 5; ++column) {
            table->setItem(index, column, new QTableWidgetItem());
        }

        auto *check = table->item(index, 0);
        check->setFlags(Qt::ItemIsEnabled | Qt::ItemIsUserCheckable | Qt::ItemIsSelectable);
        check->setCheckState(safe && source_exists ? Qt::Checked : Qt::Unchecked);
        check->setData(Qt::UserRole, hash);

        QStringList tags;

        for (const QJsonValue &tag : file.value(QStringLiteral("tags")).toArray()) {
            tags.append(tag.toString());
        }

        QString note = QStringLiteral("ready");

        if (!source_exists) {
            note = QStringLiteral("source missing");
        } else if (!safe) {
            note = QStringLiteral("unsafe filename");
        } else if (exists) {
            note = QStringLiteral("will be updated");
        }

        table->item(index, 1)->setText(hash);
        table->item(index, 2)->setText(tags.join(QStringLiteral(", ")));
        table->item(index, 3)->setText(relative);
        table->item(index, 3)->setToolTip(destination);
        table->item(index, 4)->setText(note);
    }

    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *check_all = new QPushButton(QStringLiteral("check all"), &dialog);
    auto *check_none = new QPushButton(QStringLiteral("check none"), &dialog);
    auto *export_checked = new QPushButton(QStringLiteral("export checked"), &dialog);
    auto *close = new QPushButton(QStringLiteral("close"), &dialog);
    buttons->addWidget(check_all);
    buttons->addWidget(check_none);
    buttons->addStretch(1);
    buttons->addWidget(export_checked);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    auto set_all = [table](Qt::CheckState value) {
        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && (item->flags() & Qt::ItemIsUserCheckable)) {
                item->setCheckState(value);
            }
        }
    };

    connect(check_all, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Checked); });
    connect(check_none, &QPushButton::clicked, &dialog, [set_all]() { set_all(Qt::Unchecked); });
    connect(close, &QPushButton::clicked, &dialog, &QDialog::reject);
    connect(export_checked, &QPushButton::clicked, &dialog, [this, &dialog, table, name]() {
        QJsonArray hashes;

        for (int index = 0; index < table->rowCount(); ++index) {
            QTableWidgetItem *item = table->item(index, 0);

            if (item != nullptr && item->checkState() == Qt::Checked) {
                hashes.append(item->data(Qt::UserRole).toString());
            }
        }

        if (hashes.isEmpty()) {
            QMessageBox::information(&dialog, QStringLiteral("export folder"),
                                     QStringLiteral("No files are checked."));
            return;
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("name"), name);
        payload.insert(QStringLiteral("hashes"), hashes);
        notify(QStringLiteral("export_folder_run"), active_page, json_payload(payload));
        dialog.accept();
    });

    dialog.exec();
}

void HydrusMainWindow::show_options(const QJsonObject &state) {
    HydrusOptionsDialog dialog(this, state, this);
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

void HydrusMainWindow::show_tag_filter(const QJsonObject &state) {
    const QString service_key = state.value(QStringLiteral("service_key")).toString();
    const QString name = state.value(QStringLiteral("name")).toString();
    const bool can_purge = state.value(QStringLiteral("can_purge")).toBool(false);

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("tag filter: ") + name);
    dialog.setObjectName(QStringLiteral("hydrus_tag_filter_dialog"));
    dialog.resize(760, 640);

    auto *layout = new QVBoxLayout(&dialog);

    auto *summary = new QLabel(
        QStringLiteral("current filter: %1").arg(state.value(QStringLiteral("filter_string")).toString()),
        &dialog);
    summary->setWordWrap(true);
    layout->addWidget(summary);

    const QString note = state.value(QStringLiteral("note")).toString();

    if (!note.isEmpty()) {
        auto *note_label = new QLabel(note, &dialog);
        note_label->setWordWrap(true);
        layout->addWidget(note_label);
    }

    auto *explanation = new QLabel(
        QStringLiteral("One tag slice per line. Bullet points are namespaces (\"character\" or \"character:\"); "
                       "\"*\" matches all unnamespaced tags and \"*:*\" matches all namespaced tags. "
                       "Whitelist rules are exceptions to blacklist rules."),
        &dialog);
    explanation->setWordWrap(true);
    layout->addWidget(explanation);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setObjectName(QStringLiteral("tag_filter_whitelist"));
    whitelist->setPlaceholderText(QStringLiteral("character\ncreator:someone"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setObjectName(QStringLiteral("tag_filter_blacklist"));
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*\nspecies:chat"));
    tabs->addTab(whitelist, QStringLiteral("whitelist"));
    tabs->addTab(blacklist, QStringLiteral("blacklist"));
    layout->addWidget(tabs, 1);

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &entry : state.value(QStringLiteral("rules")).toArray()) {
        const QJsonObject rule = entry.toObject();
        const QString display = rule.value(QStringLiteral("display")).toString();
        const QString kind = rule.value(QStringLiteral("rule")).toString();

        if (kind == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            QJsonObject entry;
            entry.insert(QStringLiteral("slice"), slice);
            entry.insert(QStringLiteral("rule"), rule);
            rules.append(entry);
        }

        return rules;
    };

    auto build_payload = [&]() {
        QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

        for (const QJsonValue &entry : collect_rules(blacklist, QStringLiteral("blacklist"))) {
            rules.append(entry);
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("rules"), rules);

        return payload;
    };

    auto *test_row = new QHBoxLayout();
    auto *test_input = new QLineEdit(&dialog);
    test_input->setPlaceholderText(QStringLiteral("character:samus"));
    auto *test_button = new QPushButton(QStringLiteral("test tag"), &dialog);
    test_row->addWidget(new QLabel(QStringLiteral("test:"), &dialog));
    test_row->addWidget(test_input, 1);
    test_row->addWidget(test_button);
    layout->addLayout(test_row);

    connect(test_button, &QPushButton::clicked, this, [this, test_input, build_payload]() {
        QJsonObject payload = build_payload();
        payload.insert(QStringLiteral("tag"), test_input->text().trimmed());
        notify(QStringLiteral("tag_filter_test"), QString(), json_payload(payload));
    });
    connect(test_input, &QLineEdit::returnPressed, test_button, &QPushButton::click);

    auto *purge_tab = new QWidget(&dialog);
    auto *purge_layout = new QVBoxLayout(purge_tab);
    auto *purge_note = new QLabel(
        can_purge
            ? QStringLiteral("Deletes every mapping of these tags from the local tag service.")
            : QStringLiteral("Purging is only available for local tag services in this build."),
        purge_tab);
    purge_note->setWordWrap(true);
    purge_layout->addWidget(purge_note);

    auto *purge_tags = new QPlainTextEdit(purge_tab);
    purge_tags->setPlaceholderText(QStringLiteral("character:samus\ncreator:someone"));
    purge_tags->setEnabled(can_purge);
    purge_layout->addWidget(purge_tags, 1);

    auto *reason_row = new QHBoxLayout();
    auto *purge_reason = new QLineEdit(purge_tab);
    purge_reason->setPlaceholderText(QStringLiteral("Tags purged by janitor."));
    purge_reason->setEnabled(can_purge);
    reason_row->addWidget(new QLabel(QStringLiteral("reason:"), purge_tab));
    reason_row->addWidget(purge_reason, 1);
    purge_layout->addLayout(reason_row);

    auto *purge_button = new QPushButton(QStringLiteral("purge tags"), purge_tab);
    purge_button->setEnabled(can_purge);
    auto *purge_footer = new QHBoxLayout();
    purge_footer->addStretch(1);
    purge_footer->addWidget(purge_button);
    purge_layout->addLayout(purge_footer);
    tabs->addTab(purge_tab, QStringLiteral("purge"));

    connect(purge_button, &QPushButton::clicked, this, [this, service_key, purge_tags, purge_reason, purge_button]() {
        const QStringList tags = purge_tags->toPlainText().split(QLatin1Char('\n'), Qt::SkipEmptyParts);

        if (tags.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("purge tags"), QStringLiteral("Enter tags first."));
            return;
        }

        if (QMessageBox::question(this, QStringLiteral("purge tags"),
                                  QStringLiteral("Purge %1 tag(s) from this service?").arg(tags.size()))
            != QMessageBox::Yes) {
            return;
        }

        QJsonArray list;

        for (const QString &tag : tags) {
            if (!tag.trimmed().isEmpty()) {
                list.append(tag.trimmed());
            }
        }

        QJsonObject payload;
        payload.insert(QStringLiteral("service_key"), service_key);
        payload.insert(QStringLiteral("tags"), list);
        payload.insert(QStringLiteral("reason"), purge_reason->text().trimmed());

        notify(QStringLiteral("purge_tags"), QString(), json_payload(payload));
        purge_tags->clear();
        purge_button->setEnabled(false);
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, this, [&]() {
        notify(QStringLiteral("tag_filter_save"), QString(), json_payload(build_payload()));
        dialog.accept();
    });

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_TAG_FILTER_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    dialog.exec();
}

QString api_permission_slice_display(const QString &slice) {
    if (slice.isEmpty()) {
        return QStringLiteral("*");
    }

    if (slice == QStringLiteral(":")) {
        return QStringLiteral("*:*");
    }

    return slice;
}

void show_api_permission_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const QString original = entry.value(QStringLiteral("access_key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add api permissions")
                                             : QStringLiteral("edit api permissions"));
    dialog.setObjectName(QStringLiteral("hydrus_api_permission_editor"));
    dialog.resize(680, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(entry.value(QStringLiteral("name")).toString(), &dialog);
    auto *access_key = new QLineEdit(original, &dialog);
    access_key->setReadOnly(true);
    auto *generate = new QPushButton(QStringLiteral("generate new key"), &dialog);

    auto *key_row = new QHBoxLayout();
    key_row->addWidget(access_key, 1);
    key_row->addWidget(generate);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("access key"), key_row);
    layout->addLayout(form);

    QObject::connect(generate, &QPushButton::clicked, &dialog, [access_key]() {
        QString key;

        for (int index = 0; index < 4; ++index) {
            key += QStringLiteral("%1").arg(QRandomGenerator::system()->generate64(), 16, 16, QLatin1Char('0'));
        }

        access_key->setText(key);
    });

    auto *permits_everything = new QCheckBox(QStringLiteral("permits everything"), &dialog);
    permits_everything->setChecked(entry.value(QStringLiteral("permits_everything")).toBool(true));
    layout->addWidget(permits_everything);

    auto *permission_list = new QListWidget(&dialog);
    permission_list->setObjectName(QStringLiteral("api_permission_list"));

    QJsonArray existing_permissions = entry.value(QStringLiteral("basic_permissions")).toArray();
    QSet<int> checked_permissions;

    for (const QJsonValue &value : existing_permissions) {
        checked_permissions.insert(value.toInt());
    }

    for (const QJsonValue &value : data.value(QStringLiteral("available_permissions")).toArray()) {
        const QJsonObject permission = value.toObject();
        auto *item = new QListWidgetItem(permission.value(QStringLiteral("label")).toString(), permission_list);
        item->setData(Qt::UserRole, permission.value(QStringLiteral("value")).toInt());
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setCheckState(checked_permissions.contains(permission.value(QStringLiteral("value")).toInt())
                                ? Qt::Checked
                                : Qt::Unchecked);
    }

    layout->addWidget(new QLabel(QStringLiteral("basic permissions"), &dialog));
    layout->addWidget(permission_list, 1);

    auto *tabs = new QTabWidget(&dialog);
    auto *whitelist = new QPlainTextEdit(tabs);
    whitelist->setPlaceholderText(QStringLiteral("character:"));
    auto *blacklist = new QPlainTextEdit(tabs);
    blacklist->setPlaceholderText(QStringLiteral("*\n*:*"));

    QStringList whitelist_lines;
    QStringList blacklist_lines;

    for (const QJsonValue &value : entry.value(QStringLiteral("search_tag_filter")).toArray()) {
        const QJsonArray pair = value.toArray();

        if (pair.size() != 2) {
            continue;
        }

        const QString display = api_permission_slice_display(pair.at(0).toString());

        if (pair.at(1).toString() == QStringLiteral("whitelist")) {
            whitelist_lines.append(display);
        } else {
            blacklist_lines.append(display);
        }
    }

    whitelist->setPlainText(whitelist_lines.join(QLatin1Char('\n')));
    blacklist->setPlainText(blacklist_lines.join(QLatin1Char('\n')));
    tabs->addTab(whitelist, QStringLiteral("allowed search tags"));
    tabs->addTab(blacklist, QStringLiteral("denied search tags"));
    layout->addWidget(tabs, 1);

    auto collect_rules = [](QPlainTextEdit *edit, const QString &rule) {
        QJsonArray rules;

        for (const QString &line : edit->toPlainText().split(QLatin1Char('\n'))) {
            const QString trimmed = line.trimmed();

            if (trimmed.isEmpty()) {
                continue;
            }

            QString slice = trimmed;

            if (trimmed == QStringLiteral("*")) {
                slice = QString();
            } else if (trimmed == QStringLiteral("*:*") || trimmed == QStringLiteral("*:")) {
                slice = QStringLiteral(":");
            }

            rules.append(QJsonArray{slice, rule});
        }

        return rules;
    };

    auto update_enabled = [permits_everything, permission_list]() {
        permission_list->setEnabled(!permits_everything->isChecked());
    };
    QObject::connect(permits_everything, &QCheckBox::clicked, &dialog, [update_enabled](bool) { update_enabled(); });
    update_enabled();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("Enter a name for this access key."));
            return;
        }

        if (access_key->text().trimmed().size() != 64) {
            QMessageBox::warning(&dialog, QStringLiteral("api permissions"),
                                 QStringLiteral("The access key must be 64 hex characters."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (int index = 0; index < permission_list->count(); ++index) {
        QListWidgetItem *item = permission_list->item(index);

        if (item->checkState() == Qt::Checked) {
            permissions.append(item->data(Qt::UserRole).toInt());
        }
    }

    QJsonArray rules = collect_rules(whitelist, QStringLiteral("whitelist"));

    for (const QJsonValue &rule : collect_rules(blacklist, QStringLiteral("blacklist"))) {
        rules.append(rule);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed().toLower());
    payload.insert(QStringLiteral("original_access_key"), original);
    payload.insert(QStringLiteral("permits_everything"), permits_everything->isChecked());
    payload.insert(QStringLiteral("basic_permissions"), permissions);
    payload.insert(QStringLiteral("rules"), rules);

    main->notify(QStringLiteral("api_permissions_save"), QString(), json_payload(payload));
}

HydrusApiPermissionsDialog::HydrusApiPermissionsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("client api permissions"));
    setObjectName(QStringLiteral("hydrus_api_permissions_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(860, 480);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("access key"),
        QStringLiteral("permissions")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add access key..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_entry(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_API_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 1) != nullptr
                && table->item(row, 1)->text().compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusApiPermissionsDialog::set_state(const QJsonObject &state) {
    data = state;
    status->setText(QStringLiteral("api: %1").arg(state.value(QStringLiteral("api_status")).toString()));

    const QJsonArray permissions = state.value(QStringLiteral("permissions")).toArray();
    table->setRowCount(permissions.size());

    for (int index = 0; index < permissions.size(); ++index) {
        const QJsonObject entry = permissions.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = table->item(index, 0);
        name_item->setText(entry.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, entry);
        table->item(index, 1)->setText(entry.value(QStringLiteral("access_key")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("summary")).toString());
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusApiPermissionsDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusApiPermissionsDialog::add_entry() {
    show_api_permission_editor(main, this, data, QJsonObject());
}

void HydrusApiPermissionsDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        show_api_permission_editor(main, this, data, entry);
    }
}

void HydrusApiPermissionsDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete access key"),
            QStringLiteral("Delete '%1'? Anything using that key will lose access.")
                .arg(entry.value(QStringLiteral("name")).toString()))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("api_permissions_delete"), QString(),
                 entry.value(QStringLiteral("access_key")).toString());
}

void HydrusMainWindow::show_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog == nullptr) {
        api_permissions_dialog = new HydrusApiPermissionsDialog(this, state, this);
        connect(api_permissions_dialog, &QObject::destroyed, this, [this]() { api_permissions_dialog = nullptr; });
        api_permissions_dialog->show();
    } else {
        api_permissions_dialog->set_state(state);
        api_permissions_dialog->raise();
        api_permissions_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_api_permissions(const QJsonObject &state) {
    if (api_permissions_dialog != nullptr) {
        api_permissions_dialog->set_state(state);
    }
}

void show_account_type_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &entry) {
    const bool adding = entry.isEmpty();
    const QString key = entry.value(QStringLiteral("key")).toString();

    QDialog dialog(parent);
    dialog.setWindowTitle(adding ? QStringLiteral("add account type") : QStringLiteral("edit account type"));
    dialog.setObjectName(QStringLiteral("hydrus_account_type_editor"));
    dialog.resize(640, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *title = new QLineEdit(entry.value(QStringLiteral("title")).toString(), &dialog);
    form->addRow(QStringLiteral("title"), title);

    auto *accounts = new QSpinBox(&dialog);
    accounts->setRange(0, 1000000);
    accounts->setValue(entry.value(QStringLiteral("auto_create_accounts")).toInt(0));
    auto *time_delta = new QSpinBox(&dialog);
    time_delta->setRange(60, 365 * 86400);
    time_delta->setValue(entry.value(QStringLiteral("auto_create_time_delta")).toInt(86400));
    time_delta->setSuffix(QStringLiteral(" s"));

    auto *velocity_row = new QHBoxLayout();
    velocity_row->addWidget(accounts);
    velocity_row->addWidget(new QLabel(QStringLiteral("account(s) per"), &dialog));
    velocity_row->addWidget(time_delta);
    form->addRow(QStringLiteral("auto-creation"), velocity_row);
    layout->addLayout(form);

    QHash<int, QComboBox *> permission_controls;
    QJsonArray entry_permissions = entry.value(QStringLiteral("permissions")).toArray();
    QHash<int, int> current_actions;

    for (const QJsonValue &value : entry_permissions) {
        const QJsonArray pair = value.toArray();

        if (pair.size() == 2) {
            current_actions.insert(pair.at(0).toInt(), pair.at(1).isNull() ? -1 : pair.at(1).toInt());
        }
    }

    auto *permissions_box = new QGroupBox(QStringLiteral("permissions"), &dialog);
    auto *permissions_form = new QFormLayout(permissions_box);

    for (const QJsonValue &value : data.value(QStringLiteral("possible")).toArray()) {
        const QJsonObject content_type = value.toObject();
        auto *combo = new QComboBox(permissions_box);
        combo->setObjectName(QStringLiteral("account_type_permission_%1")
                                 .arg(content_type.value(QStringLiteral("content_type")).toInt()));

        for (const QJsonValue &action_value : content_type.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject action = action_value.toObject();
            const QJsonValue action_number = action.value(QStringLiteral("value"));

            combo->addItem(action.value(QStringLiteral("label")).toString(),
                           action_number.isNull() ? -1 : action_number.toInt());
        }

        const int wanted = current_actions.value(content_type.value(QStringLiteral("content_type")).toInt(), -1);
        const int index = combo->findData(wanted);
        combo->setCurrentIndex(index >= 0 ? index : 0);
        permissions_form->addRow(content_type.value(QStringLiteral("label")).toString(), combo);
        permission_controls.insert(content_type.value(QStringLiteral("content_type")).toInt(), combo);
    }

    layout->addWidget(permissions_box, 1);

    auto *note = new QLabel(
        QStringLiteral("Bandwidth rules and auto-creation history are preserved. "
                       "Permissions without an entry mean no permission."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (title->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("account type"),
                                 QStringLiteral("Enter a title."));
            return;
        }

        dialog.accept();
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonArray permissions;

    for (auto iterator = permission_controls.constBegin(); iterator != permission_controls.constEnd(); ++iterator) {
        const int action = iterator.value()->currentData().toInt();

        if (action < 0) {
            continue;
        }

        permissions.append(QJsonArray{iterator.key(), action});
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), key);
    payload.insert(QStringLiteral("title"), title->text().trimmed());
    payload.insert(QStringLiteral("permissions"), permissions);
    payload.insert(QStringLiteral("auto_create_accounts"), accounts->value());
    payload.insert(QStringLiteral("auto_create_time_delta"), time_delta->value());

    main->notify(QStringLiteral("account_type_save"), QString(), json_payload(payload));
}

HydrusAccountTypesDialog::HydrusAccountTypesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("account types"));
    setObjectName(QStringLiteral("hydrus_account_types_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 480);

    auto *layout = new QVBoxLayout(this);

    table = new QTableWidget(this);
    table->setColumnCount(3);
    table->setHorizontalHeaderLabels({
        QStringLiteral("title"),
        QStringLiteral("permissions"),
        QStringLiteral("auto-creation")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add account type..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("delete"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_entry(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_ACCOUNT_TYPE_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_entry();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (table->item(row, 0) != nullptr
                && table->item(row, 0)->data(Qt::UserRole).toJsonObject().value(QStringLiteral("key")).toString()
                       == edit_key) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }
}

void HydrusAccountTypesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray types = state.value(QStringLiteral("types")).toArray();
    table->setRowCount(types.size());

    for (int index = 0; index < types.size(); ++index) {
        const QJsonObject entry = types.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *title_item = table->item(index, 0);
        title_item->setText(entry.value(QStringLiteral("title")).toString());
        title_item->setData(Qt::UserRole, entry);

        if (entry.value(QStringLiteral("is_null")).toBool()) {
            title_item->setForeground(QColor(120, 150, 190));
        }

        table->item(index, 1)->setText(entry.value(QStringLiteral("summary")).toString());

        const int accounts = entry.value(QStringLiteral("auto_create_accounts")).toInt();
        table->item(index, 2)->setText(accounts == 0
                                           ? QStringLiteral("disabled")
                                           : QStringLiteral("%1 / %2 s")
                                                 .arg(accounts)
                                                 .arg(entry.value(QStringLiteral("auto_create_time_delta")).toInt()));
    }

    table->resizeRowsToContents();
}

QJsonObject HydrusAccountTypesDialog::selected_entry() const {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return {};
    }

    return table->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

void HydrusAccountTypesDialog::add_entry() {
    show_account_type_editor(main, this, data, QJsonObject());
}

void HydrusAccountTypesDialog::edit_selected() {
    const QJsonObject entry = selected_entry();

    if (!entry.isEmpty()) {
        if (entry.value(QStringLiteral("is_null")).toBool()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("The null account type cannot be edited."));
            return;
        }

        show_account_type_editor(main, this, data, entry);
    }
}

void HydrusAccountTypesDialog::delete_selected() {
    const QJsonObject entry = selected_entry();

    if (entry.isEmpty()) {
        return;
    }

    if (entry.value(QStringLiteral("is_null")).toBool()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("The null account type cannot be deleted."));
        return;
    }

    QStringList titles;
    QStringList keys;

    for (const QJsonValue &value : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject other = value.toObject();

        if (other.value(QStringLiteral("key")).toString() == entry.value(QStringLiteral("key")).toString()
            || other.value(QStringLiteral("is_null")).toBool()) {
            continue;
        }

        titles.append(other.value(QStringLiteral("title")).toString());
        keys.append(other.value(QStringLiteral("key")).toString());
    }

    if (titles.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("account type"),
                                 QStringLiteral("There is no other account type to move accounts to."));
        return;
    }

    bool ok = false;
    const QString title = QInputDialog::getItem(
        this, QStringLiteral("delete account type"),
        QStringLiteral("Move its accounts to:"), titles, 0, false, &ok);

    if (!ok) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("account_type_key"), entry.value(QStringLiteral("key")));
    payload.insert(QStringLiteral("replacement_account_type_key"), keys.at(titles.indexOf(title)));

    main->notify(QStringLiteral("account_type_delete"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_account_types(const QJsonObject &state) {
    if (account_types_dialog == nullptr) {
        account_types_dialog = new HydrusAccountTypesDialog(this, state, this);
        connect(account_types_dialog, &QObject::destroyed, this, [this]() { account_types_dialog = nullptr; });
        account_types_dialog->show();
    } else {
        account_types_dialog->set_state(state);
        account_types_dialog->raise();
        account_types_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_account_types(const QJsonObject &state) {
    if (account_types_dialog != nullptr) {
        account_types_dialog->set_state(state);
    }
}

HydrusOrphanFilesDialog::HydrusOrphanFilesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("clear orphan files"));
    setObjectName(QStringLiteral("hydrus_orphan_files_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(880, 520);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Files in the media storage whose hash is not in the database. "
                       "Thumbnails are included. Anything that is not named after a hash is left alone."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    status = new QLabel(this);
    status->setObjectName(QStringLiteral("orphan_files_status"));
    layout->addWidget(status);

    table = new QTableWidget(this);
    table->setColumnCount(4);
    table->setHorizontalHeaderLabels({
        QStringLiteral("kind"),
        QStringLiteral("size"),
        QStringLiteral("hash"),
        QStringLiteral("path")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Interactive);
    table->setColumnWidth(2, 260);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setWordWrap(false);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *rescan = new QPushButton(QStringLiteral("scan again"), this);
    auto *delete_selected_button = new QPushButton(QStringLiteral("delete selected"), this);
    auto *delete_all_button = new QPushButton(QStringLiteral("delete all"), this);
    auto *move_selected_button = new QPushButton(QStringLiteral("move selected to..."), this);
    auto *move_all_button = new QPushButton(QStringLiteral("move all to..."), this);
    buttons->addWidget(rescan);
    buttons->addWidget(delete_selected_button);
    buttons->addWidget(delete_all_button);
    buttons->addWidget(move_selected_button);
    buttons->addWidget(move_all_button);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(rescan, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("orphan_files_scan"));
    });
    connect(delete_selected_button, &QPushButton::clicked, this, [this]() { delete_selected(false); });
    connect(delete_all_button, &QPushButton::clicked, this, [this]() { delete_selected(true); });
    connect(move_selected_button, &QPushButton::clicked, this, [this]() { move_selected(false); });
    connect(move_all_button, &QPushButton::clicked, this, [this]() { move_selected(true); });

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusOrphanFilesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray orphans = state.value(QStringLiteral("orphans")).toArray();
    const int previous_row = table->currentRow();
    table->setRowCount(orphans.size());

    for (int index = 0; index < orphans.size(); ++index) {
        const QJsonObject entry = orphans.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *kind = table->item(index, 0);
        kind->setText(entry.value(QStringLiteral("kind")).toString());
        kind->setData(Qt::UserRole, entry.value(QStringLiteral("path")).toString());
        table->item(index, 1)->setText(
            entry.value(QStringLiteral("size_display")).toString());
        table->item(index, 2)->setText(entry.value(QStringLiteral("hash")).toString());
        table->item(index, 3)->setText(entry.value(QStringLiteral("path")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    table->resizeRowsToContents();
    status->setText(QStringLiteral("%1 — %2 file(s), %3 thumbnail(s), %4")
                        .arg(state.value(QStringLiteral("status")).toString())
                        .arg(state.value(QStringLiteral("files")).toInt())
                        .arg(state.value(QStringLiteral("thumbnails")).toInt())
                        .arg(state.value(QStringLiteral("bytes_display")).toString()));
}

QStringList HydrusOrphanFilesDialog::selected_paths(bool all) const {
    QStringList paths;

    if (all) {
        for (int row = 0; row < table->rowCount(); ++row) {
            paths.append(table->item(row, 0)->data(Qt::UserRole).toString());
        }

        return paths;
    }

    for (const QModelIndex &index : table->selectionModel()->selectedRows()) {
        paths.append(table->item(index.row(), 0)->data(Qt::UserRole).toString());
    }

    return paths;
}

void HydrusOrphanFilesDialog::delete_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to delete first."));
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete orphan files"),
                              QStringLiteral("Permanently delete %1 orphan file(s)? This cannot be undone.")
                                  .arg(paths.size()))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    main->notify(QStringLiteral("orphan_files_delete"), QString(), json_payload(payload));
}

void HydrusOrphanFilesDialog::move_selected(bool all) {
    const QStringList paths = selected_paths(all);

    if (paths.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("clear orphan files"),
                                 QStringLiteral("Select the orphan files to move first."));
        return;
    }

    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("move orphan files to"),
        QStandardPaths::writableLocation(QStandardPaths::DownloadLocation));

    if (directory.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("paths"), QJsonArray::fromStringList(paths));
    payload.insert(QStringLiteral("target"), directory);
    main->notify(QStringLiteral("orphan_files_move"), QString(), json_payload(payload));
}

void HydrusMainWindow::show_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog == nullptr) {
        orphan_files_dialog = new HydrusOrphanFilesDialog(this, state, this);
        connect(orphan_files_dialog, &QObject::destroyed, this, [this]() { orphan_files_dialog = nullptr; });
        orphan_files_dialog->show();
    } else {
        orphan_files_dialog->set_state(state);
        orphan_files_dialog->raise();
        orphan_files_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_orphan_files(const QJsonObject &state) {
    if (orphan_files_dialog != nullptr) {
        orphan_files_dialog->set_state(state);
    }
}

HydrusDatabaseBackupDialog::HydrusDatabaseBackupDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("database backup"));
    setObjectName(QStringLiteral("hydrus_database_backup_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(640, 260);

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("A consistent copy of client.db is written as client.db in the backup directory. "
                       "The existing copy is replaced."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *form = new QFormLayout();
    auto *path_row = new QHBoxLayout();
    path = new QLineEdit(this);
    path->setReadOnly(true);
    auto *change = new QPushButton(QStringLiteral("change path..."), this);
    path_row->addWidget(path, 1);
    path_row->addWidget(change);
    form->addRow(QStringLiteral("backup directory"), path_row);
    last_backup = new QLabel(this);
    form->addRow(QStringLiteral("last backup"), last_backup);
    layout->addLayout(form);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);
    layout->addStretch(1);

    auto *buttons = new QHBoxLayout();
    auto *run = new QPushButton(QStringLiteral("back up now"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(run);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    connect(change, &QPushButton::clicked, this, [this]() { choose_path(); });
    connect(run, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("database backup"),
                                  QStringLiteral("Back up the database now?"))
            == QMessageBox::Yes) {
            main->notify(QStringLiteral("db_backup_run"));
        }
    });
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDatabaseBackupDialog::set_state(const QJsonObject &state) {
    const QString directory = state.value(QStringLiteral("path")).toString();
    path->setText(directory.isEmpty() ? QStringLiteral("(not set)") : directory);
    path->setCursorPosition(0);
    last_backup->setText(state.value(QStringLiteral("last_backup")).toString());

    const QString target = state.value(QStringLiteral("target")).toString();
    status->setText(target.isEmpty()
                        ? QStringLiteral("Set a backup directory to enable backups.")
                        : QStringLiteral("target: %1").arg(target));
}

void HydrusDatabaseBackupDialog::choose_path() {
    const QString directory = QFileDialog::getExistingDirectory(
        this, QStringLiteral("backup directory"),
        QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation));

    if (!directory.isEmpty()) {
        main->notify(QStringLiteral("db_backup_set_path"), QString(), directory);
    }
}

void HydrusMainWindow::show_database_backup(const QJsonObject &state) {
    if (database_backup_dialog == nullptr) {
        database_backup_dialog = new HydrusDatabaseBackupDialog(this, state, this);
        connect(database_backup_dialog, &QObject::destroyed, this, [this]() { database_backup_dialog = nullptr; });
        database_backup_dialog->show();
    } else {
        database_backup_dialog->set_state(state);
        database_backup_dialog->raise();
        database_backup_dialog->activateWindow();
    }
}

void HydrusMainWindow::update_database_backup(const QJsonObject &state) {
    if (database_backup_dialog != nullptr) {
        database_backup_dialog->set_state(state);
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

