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
// HydrusDownloaderPage
// ---------------------------------------------------------------------------

HydrusDownloaderPage::HydrusDownloaderPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    kind = state.value(QStringLiteral("kind")).toString();

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *splitter = new QSplitter(Qt::Horizontal, this);

    sidebar_scroll = new QScrollArea(splitter);
    sidebar_scroll->setWidgetResizable(true);
    sidebar_scroll->setFrameShape(QFrame::NoFrame);
    sidebar_scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);

    sidebar_content = new QWidget();
    auto *sidebar_layout = new QVBoxLayout(sidebar_content);
    sidebar_layout->setContentsMargins(6, 6, 6, 6);
    sidebar_layout->setSpacing(4);
    sidebar_scroll->setWidget(sidebar_content);
    splitter->addWidget(sidebar_scroll);

    results = new QWidget(splitter);
    results->setAutoFillBackground(true);
    QPalette results_palette = results->palette();
    results_palette.setColor(QPalette::Window, Qt::white);
    results->setPalette(results_palette);
    auto *results_layout = new QVBoxLayout(results);
    results_layout->setContentsMargins(0, 0, 0, 0);
    results_layout->setSpacing(0);
    splitter->addWidget(results);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({412, 884});

    layout->addWidget(splitter);

    const HydrusSortOption *initial_sort = find_sort_option(QStringLiteral("size"));

    sort_type = new QPushButton(QString::fromUtf8(initial_sort->label), sidebar_content);
    sort_type->setMinimumHeight(24);
    sort_type->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);

    sort_order = new QPushButton(QString::fromUtf8(initial_sort->ascending), sidebar_content);
    sort_order->setMinimumHeight(24);
    sort_order->setMinimumWidth(90);

    auto *sort_row = new QWidget(sidebar_content);
    sort_row->setFixedHeight(24);
    auto *sort_row_layout = new QHBoxLayout(sort_row);
    sort_row_layout->setContentsMargins(0, 0, 0, 0);
    sort_row_layout->setSpacing(2);
    sort_row_layout->addWidget(sort_type, 1);
    sort_row_layout->addWidget(sort_order, 0);
    sidebar_layout->addWidget(sort_row);

    auto *sort_menu = new QMenu(this);

    for (const HydrusSortOption &option : hydrus_sort_options) {
        auto *action = sort_menu->addAction(QString::fromUtf8(option.label));
        action->setData(QString::fromUtf8(option.key));
    }

    const auto set_sort = [this](const QString &key) {
        const HydrusSortOption *option = find_sort_option(key);
        sort_type->setText(QString::fromUtf8(option->label));
        sort_order->setText(QString::fromUtf8(option->ascending));
    };

    const auto set_sort_direction = [this](bool ascending) {
        for (const HydrusSortOption &option : hydrus_sort_options) {
            if (sort_type->text() == QString::fromUtf8(option.label)) {
                sort_order->setText(
                    QString::fromUtf8(ascending ? option.ascending : option.descending));
                break;
            }
        }
    };

    connect(sort_menu, &QMenu::triggered, this, [set_sort](QAction *action) {
        set_sort(action->data().toString());
    });

    connect(sort_type, &QPushButton::clicked, this, [this, sort_menu]() {
        sort_menu->popup(sort_type->mapToGlobal(QPoint(0, sort_type->height())));
    });

    connect(sort_order, &QPushButton::clicked, this, [this, set_sort_direction]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        auto *ascending = menu->addAction(QStringLiteral("smallest first"));
        auto *descending = menu->addAction(QStringLiteral("largest first"));

        connect(ascending, &QAction::triggered, this, [set_sort_direction]() {
            set_sort_direction(true);
        });
        connect(descending, &QAction::triggered, this, [set_sort_direction]() {
            set_sort_direction(false);
        });

        menu->popup(sort_order->mapToGlobal(QPoint(0, sort_order->height())));
    });

    collect = new QComboBox(sidebar_content);
    collect->addItem(QStringLiteral("no collections"));
    collect->setFixedHeight(24);

    collect_cog = new QToolButton(sidebar_content);
    collect_cog->setIcon(hydrus_small_icon(QStringLiteral("gear")));
    collect_cog->setIconSize(QSize(16, 16));
    collect_cog->setToolTip(QStringLiteral("collection options"));
    collect_cog->setAutoRaise(true);
    collect_cog->setFixedSize(24, 24);
    collect_cog->setFocusPolicy(Qt::NoFocus);

    auto *collect_row = new QWidget(sidebar_content);
    collect_row->setFixedHeight(24);
    auto *collect_row_layout = new QHBoxLayout(collect_row);
    collect_row_layout->setContentsMargins(0, 0, 0, 0);
    collect_row_layout->setSpacing(2);
    collect_row_layout->addWidget(collect, 1);
    collect_row_layout->addWidget(collect_cog, 0);
    sidebar_layout->addWidget(collect_row);

    connect(collect_cog, &QToolButton::clicked, this, [this]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        auto *nothing = menu->addAction(QStringLiteral("no collection options available"));
        nothing->setEnabled(false);
        menu->popup(collect_cog->mapToGlobal(QPoint(0, collect_cog->height())));
    });

    const auto row_icon = [this](QWidget *parent, const QString &icon_kind, const QString &tooltip) {
        auto *button = new QToolButton(parent);
        button->setIcon(hydrus_small_icon(icon_kind));
        button->setIconSize(QSize(16, 16));
        button->setFixedSize(28, 24);
        button->setAutoRaise(true);
        button->setFocusPolicy(Qt::NoFocus);
        button->setToolTip(tooltip);
        return button;
    };

    const auto make_status_box = [this, row_icon](
                                     QWidget *container,
                                     QVBoxLayout *outer_layout,
                                     const QString &title,
                                     QLabel **status_label,
                                     QLabel **seed_label,
                                     QToolButton **pause_button,
                                     QToolButton **log_button,
                                     QProgressBar **progress_bar,
                                     QLabel **percent_label,
                                     QProgressBar **network_bar,
                                     QLabel **network_percent,
                                     QToolButton **cog_button,
                                     const QString &pause_tip,
                                     const QString &log_text,
                                     const QString &log_action) {
        QVBoxLayout *box_layout = nullptr;
        QFrame *box = make_static_box(title, container, &box_layout);
        box_layout->setContentsMargins(8, 4, 8, 10);
        box_layout->setSpacing(8);

        auto *status_row = new QWidget(box);
        status_row->setFixedHeight(24);
        auto *status_layout = new QHBoxLayout(status_row);
        status_layout->setContentsMargins(0, 0, 0, 0);
        status_layout->setSpacing(2);

        *status_label = new QLabel(status_row);
        status_layout->addWidget(*status_label, 1);

        *pause_button = row_icon(status_row, QStringLiteral("pause"), pause_tip);
        status_layout->addWidget(*pause_button, 0);
        box_layout->addWidget(status_row);

        auto *seed_row = new QWidget(box);
        seed_row->setFixedHeight(24);
        auto *seed_layout = new QHBoxLayout(seed_row);
        seed_layout->setContentsMargins(0, 0, 0, 0);
        seed_layout->setSpacing(2);

        *seed_label = new QLabel(seed_row);
        seed_layout->addWidget(*seed_label, 1);

        *log_button = new QToolButton(seed_row);
        (*log_button)->setText(QStringLiteral("%1 log").arg(log_text));
        (*log_button)->setMinimumWidth(78);
        (*log_button)->setToolButtonStyle(Qt::ToolButtonTextOnly);
        (*log_button)->setPopupMode(QToolButton::InstantPopup);
        (*log_button)->setAutoRaise(false);
        (*log_button)->setFocusPolicy(Qt::NoFocus);

        auto *log_menu = new QMenu(*log_button);
        log_menu->addAction(
            QStringLiteral("open %1 log").arg(log_text), this, [this, log_action]() {
                main->notify(log_action, id);
            });
        log_menu->addAction(QStringLiteral("clear finished"), this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        (*log_button)->setMenu(log_menu);
        seed_layout->addWidget(*log_button, 0);
        box_layout->addWidget(seed_row);

        auto *progress_row = new QWidget(box);
        auto *progress_layout = new QHBoxLayout(progress_row);
        progress_layout->setContentsMargins(0, 0, 0, 0);
        progress_layout->setSpacing(2);

        *progress_bar = new QProgressBar(progress_row);
        (*progress_bar)->setRange(0, 100);
        (*progress_bar)->setValue(0);
        (*progress_bar)->setTextVisible(false);
        (*progress_bar)->setFixedHeight(20);
        progress_layout->addWidget(*progress_bar, 1);

        *percent_label = new QLabel(progress_row);
        (*percent_label)->setFixedWidth(38);
        (*percent_label)->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        (*percent_label)->setVisible(false);
        progress_layout->addWidget(*percent_label, 0);
        box_layout->addWidget(progress_row);

        auto *network_row = new QWidget(box);
        network_row->setFixedHeight(44);
        auto *network_layout = new QHBoxLayout(network_row);
        network_layout->setContentsMargins(0, 0, 0, 0);
        network_layout->setSpacing(2);

        *network_bar = new QProgressBar(network_row);
        (*network_bar)->setRange(0, 100);
        (*network_bar)->setValue(0);
        (*network_bar)->setTextVisible(false);
        (*network_bar)->setFixedHeight(20);
        network_layout->addWidget(*network_bar, 1);

        *network_percent = new QLabel(QStringLiteral("0%"), network_row);
        (*network_percent)->setFixedWidth(38);
        (*network_percent)->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        network_layout->addWidget(*network_percent, 0);

        *cog_button = row_icon(network_row, QStringLiteral("gear"), QStringLiteral("network job options"));
        network_layout->addWidget(*cog_button, 0);
        box_layout->addWidget(network_row);

        outer_layout->addWidget(box);

        return box;
    };

    const bool watcher = kind == QStringLiteral("watcher");
    const bool gallery = kind == QStringLiteral("gallery");
    const bool urls = kind == QStringLiteral("urls");
    const bool simple = kind == QStringLiteral("simple");

    QString box_title = QStringLiteral("url downloader");

    if (gallery) {
        box_title = QStringLiteral("gallery downloader");
    } else if (simple) {
        box_title = QStringLiteral("simple downloader");
    } else if (watcher) {
        box_title = QStringLiteral("watchers");
    }

    QVBoxLayout *main_layout = nullptr;
    QPushButton *main_expand = nullptr;
    main_box = make_static_box(box_title, sidebar_content, &main_layout, &main_expand);
    main_content = main_layout->parentWidget();
    sidebar_layout->addWidget(main_box);

    connect(main_expand, &QPushButton::clicked, this, [this, main_expand]() {
        const bool show = !main_content->isVisible();
        main_content->setVisible(show);
        main_expand->setIcon(
            style()->standardIcon(show ? QStyle::SP_ArrowUp : QStyle::SP_ArrowDown));
    });

    if (gallery) {
        status_top = new QLabel(main_content);
        status_top->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_top);

        status_bottom = new QLabel(main_content);
        status_bottom->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_bottom);

        queue = new QTableWidget(main_content);
        queue->setColumnCount(7);
        queue->setHorizontalHeaderLabels({QStringLiteral("query"),
                                          QStringLiteral("source"),
                                          QStringLiteral("files status"),
                                          QStringLiteral("search status"),
                                          QStringLiteral("status"),
                                          QStringLiteral("items"),
                                          QStringLiteral("added")});
        queue->verticalHeader()->setVisible(false);
        queue->setSelectionBehavior(QAbstractItemView::SelectRows);
        queue->setSelectionMode(QAbstractItemView::SingleSelection);
        queue->setEditTriggers(QAbstractItemView::NoEditTriggers);
        queue->setHorizontalScrollMode(QAbstractItemView::ScrollPerPixel);
        queue->setFixedHeight(128);

        queue->setSortingEnabled(true);
        queue->setTextElideMode(Qt::ElideRight);
        queue->horizontalHeader()->setTextElideMode(Qt::ElideRight);
        queue->horizontalHeader()->setFixedHeight(24);

        const int widths[] = {56, 78, 30, 30, 56, 64, 56};

        for (int index = 0; index < 7; ++index) {
            queue->setColumnWidth(index, widths[index]);
            queue->horizontalHeader()->setSectionResizeMode(index, QHeaderView::Fixed);
        }

        queue->sortByColumn(0, Qt::AscendingOrder);

        main_layout->addWidget(queue);

        auto *icon_row = new QWidget(main_content);
        auto *icon_layout = new QHBoxLayout(icon_row);
        icon_layout->setContentsMargins(0, 0, 0, 0);
        icon_layout->setSpacing(2);
        icon_layout->addStretch(1);

        auto *highlight_button = row_icon(icon_row, QStringLiteral("highlight"), QStringLiteral("highlight"));
        auto *clear_highlight_button =
            row_icon(icon_row, QStringLiteral("unhighlight"), QStringLiteral("clear highlight"));
        files_pause = row_icon(icon_row, QStringLiteral("pause"), QStringLiteral("pause/play files"));
        search_pause = row_icon(icon_row, QStringLiteral("pause"), QStringLiteral("pause/play search"));
        auto *retry_button = row_icon(icon_row, QStringLiteral("retry"), QStringLiteral("retry commands"));
        auto *remove_button = row_icon(icon_row, QStringLiteral("trash"), QStringLiteral("remove selected"));

        for (QToolButton *button : {highlight_button,
                                    clear_highlight_button,
                                    files_pause,
                                    search_pause,
                                    retry_button,
                                    remove_button}) {
            icon_layout->addWidget(button, 0);
        }

        main_layout->addWidget(icon_row);

        connect(highlight_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_open_results"), id);
        });
        connect(clear_highlight_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        connect(files_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(search_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(retry_button, &QToolButton::clicked, this, [this]() { start(); });
        connect(remove_button, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        connect(queue, &QTableWidget::cellDoubleClicked, this, [this](int, int) { start(); });

        query_row = new QWidget(main_content);
        auto *query_layout = new QHBoxLayout(query_row);
        query_layout->setContentsMargins(0, 0, 0, 0);
        query_layout->setSpacing(2);

        query = new QLineEdit(query_row);
        query->setMinimumHeight(24);
        query->setPlaceholderText(QStringLiteral("unknown downloader"));
        query_layout->addWidget(query, 1);

        auto *paste_button = row_icon(query_row, QStringLiteral("paste"), QStringLiteral("paste"));
        paste_button->setFixedSize(28, 24);
        query_layout->addWidget(paste_button, 0);

        auto *query_cog = row_icon(query_row, QStringLiteral("gear"), QStringLiteral("downloader options"));
        query_layout->addWidget(query_cog, 0);
        main_layout->addWidget(query_row);

        connect(query, &QLineEdit::returnPressed, this, [this]() { start(); });
        connect(paste_button, &QToolButton::clicked, this, [this]() {
            const QString text = QApplication::clipboard()->text().trimmed();

            if (!text.isEmpty()) {
                query->setText(text.split('\n').first().trimmed());
                start();
            }
        });

        connect(query_cog, &QToolButton::clicked, this, [this, query_cog]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            menu->addAction(QStringLiteral("start query"), this, [this]() { start(); });
            menu->addAction(QStringLiteral("clear finished"), this, [this]() {
                main->notify(QStringLiteral("downloader_clear"), id);
            });
            menu->addAction(QStringLiteral("file seed cache..."), this, [this]() {
                main->notify(QStringLiteral("file_seed_cache"), id);
            });
            menu->addAction(QStringLiteral("gallery log..."), this, [this]() {
                main->notify(QStringLiteral("gallery_seed_log"), id);
            });
            menu->popup(query_cog->mapToGlobal(QPoint(0, query_cog->height())));
        });

        gug_button = new QPushButton(QStringLiteral("no downloader set"), main_content);
        gug_button->setMinimumHeight(24);
        main_layout->addWidget(gug_button);

        site = new QComboBox(main_content);
        site->hide();

        connect(gug_button, &QPushButton::clicked, this, [this]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            auto *none = menu->addAction(QStringLiteral("no downloader set"));

            connect(none, &QAction::triggered, this, [this]() {
                site->setCurrentIndex(0);
                gug_button->setText(QStringLiteral("no downloader set"));
                url->clear();
                main->notify(QStringLiteral("downloader_set_site"), id, QString());
            });

            for (int index = 1; index < site->count(); ++index) {
                const QString title = site->itemText(index);
                auto *action = menu->addAction(title);

                connect(action, &QAction::triggered, this, [this, index, title]() {
                    site->setCurrentIndex(index);
                    gug_button->setText(title);
                    url->setText(site->itemData(index, Qt::UserRole + 1).toString());
                    main->notify(
                        QStringLiteral("downloader_set_site"),
                        id,
                        site->itemData(index).toString());
                });
            }

            menu->popup(gug_button->mapToGlobal(QPoint(0, gug_button->height())));
        });

        file_limit_row = new QWidget(main_content);
        auto *limit_layout = new QHBoxLayout(file_limit_row);
        limit_layout->setContentsMargins(0, 0, 0, 0);
        limit_layout->setSpacing(4);

        auto *limit_label = new QLabel(QStringLiteral("stop after this many files:"), file_limit_row);
        limit_layout->addWidget(limit_label, 0);

        file_limit = new QSpinBox(file_limit_row);
        file_limit->setRange(1, 10000000);
        file_limit->setValue(2000);
        file_limit->setFixedHeight(20);
        limit_layout->addWidget(file_limit, 1);

        no_limit = new QCheckBox(QStringLiteral("no limit"), file_limit_row);
        limit_layout->addWidget(no_limit, 0);
        main_layout->addWidget(file_limit_row);
    } else if (urls) {
        // Python's SidebarImporterURLs: the "url downloader" box holds an
        // "imports" box (file seed cache and file network job), a "search" box
        // (gallery seed log and gallery network job), the url input and the
        // import options button.
        QVBoxLayout *imports_layout = nullptr;
        imports_box = make_static_box(QStringLiteral("imports"), main_content, &imports_layout);
        imports_layout->setContentsMargins(8, 4, 8, 10);
        imports_layout->setSpacing(8);

        auto *pause_row = new QWidget(imports_box);
        auto *pause_row_layout = new QHBoxLayout(pause_row);
        pause_row_layout->setContentsMargins(0, 0, 0, 0);
        pause_row_layout->setSpacing(2);
        pause_row_layout->addStretch(1);
        files_pause =
            row_icon(pause_row, QStringLiteral("pause"), QStringLiteral("pause/play files"));
        pause_row_layout->addWidget(files_pause, 0);
        imports_layout->addWidget(pause_row);

        auto *file_seed_frame = new QFrame(imports_box);
        file_seed_frame->setFrameStyle(QFrame::Box | QFrame::Raised);
        auto *file_seed_layout = new QVBoxLayout(file_seed_frame);
        file_seed_layout->setContentsMargins(6, 4, 6, 4);
        file_seed_layout->setSpacing(2);

        imports_status = new QLabel(file_seed_frame);
        imports_status->setTextFormat(Qt::PlainText);
        file_seed_layout->addWidget(imports_status);

        auto *file_log_row = new QWidget(file_seed_frame);
        auto *file_log_layout = new QHBoxLayout(file_log_row);
        file_log_layout->setContentsMargins(0, 0, 0, 0);
        file_log_layout->setSpacing(2);

        file_seed_status = new QLabel(file_log_row);
        file_seed_status->setTextFormat(Qt::PlainText);
        file_log_layout->addWidget(file_seed_status, 1);

        file_log_button = new QToolButton(file_log_row);
        file_log_button->setText(QStringLiteral("file log"));
        file_log_button->setToolButtonStyle(Qt::ToolButtonTextOnly);
        file_log_button->setPopupMode(QToolButton::InstantPopup);
        file_log_button->setAutoRaise(false);
        file_log_button->setFocusPolicy(Qt::NoFocus);
        auto *file_log_menu = new QMenu(file_log_button);
        file_log_menu->addAction(QStringLiteral("open file log"), this, [this]() {
            main->notify(QStringLiteral("file_seed_cache"), id);
        });
        file_log_menu->addAction(QStringLiteral("clear finished"), this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        file_log_button->setMenu(file_log_menu);
        file_log_layout->addWidget(file_log_button, 0);
        file_seed_layout->addWidget(file_log_row);

        file_progress = new QProgressBar(file_seed_frame);
        file_progress->setRange(0, 100);
        file_progress->setValue(0);
        file_progress->setTextVisible(false);
        file_progress->setFixedHeight(18);
        file_seed_layout->addWidget(file_progress);

        imports_layout->addWidget(file_seed_frame);

        auto *file_net_frame = new QFrame(imports_box);
        file_net_frame->setFrameStyle(QFrame::Box | QFrame::Raised);
        auto *file_net_layout = new QVBoxLayout(file_net_frame);
        file_net_layout->setContentsMargins(6, 4, 6, 4);
        file_net_layout->setSpacing(2);

        auto *file_net_text_row = new QWidget(file_net_frame);
        auto *file_net_text_layout = new QHBoxLayout(file_net_text_row);
        file_net_text_layout->setContentsMargins(0, 0, 0, 0);
        file_net_text_layout->setSpacing(2);
        auto *file_net_left = new QLabel(file_net_text_row);
        file_net_left->setTextFormat(Qt::PlainText);
        file_net_text_layout->addWidget(file_net_left, 1);
        auto *file_net_right = new QLabel(file_net_text_row);
        file_net_right->setTextFormat(Qt::PlainText);
        file_net_text_layout->addWidget(file_net_right, 0);
        file_net_layout->addWidget(file_net_text_row);

        auto *file_net_gauge_row = new QWidget(file_net_frame);
        auto *file_net_gauge_layout = new QHBoxLayout(file_net_gauge_row);
        file_net_gauge_layout->setContentsMargins(0, 0, 0, 0);
        file_net_gauge_layout->setSpacing(2);
        file_network_progress = new QProgressBar(file_net_gauge_row);
        file_network_progress->setRange(0, 100);
        file_network_progress->setValue(0);
        file_network_progress->setFormat(QStringLiteral("%p%"));
        file_network_progress->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        file_network_progress->setFixedHeight(20);
        file_net_gauge_layout->addWidget(file_network_progress, 1);
        file_cancel = row_icon(file_net_gauge_row, QStringLiteral("stop"), QStringLiteral("cancel"));
        file_net_gauge_layout->addWidget(file_cancel, 0);
        file_cog = row_icon(
            file_net_gauge_row, QStringLiteral("gear"), QStringLiteral("network job options"));
        file_net_gauge_layout->addWidget(file_cog, 0);
        file_net_layout->addWidget(file_net_gauge_row);

        imports_layout->addWidget(file_net_frame);
        main_layout->addWidget(imports_box);

        QVBoxLayout *search_layout = nullptr;
        search_box = make_static_box(QStringLiteral("search"), main_content, &search_layout);
        search_layout->setContentsMargins(8, 4, 8, 10);
        search_layout->setSpacing(8);

        auto *search_log_row = new QWidget(search_box);
        auto *search_log_layout = new QHBoxLayout(search_log_row);
        search_log_layout->setContentsMargins(0, 0, 0, 0);
        search_log_layout->setSpacing(2);

        search_status = new QLabel(search_log_row);
        search_status->setTextFormat(Qt::PlainText);
        search_log_layout->addWidget(search_status, 1);

        search_log_button = new QToolButton(search_log_row);
        search_log_button->setText(QStringLiteral("search log"));
        search_log_button->setToolButtonStyle(Qt::ToolButtonTextOnly);
        search_log_button->setPopupMode(QToolButton::InstantPopup);
        search_log_button->setAutoRaise(false);
        search_log_button->setFocusPolicy(Qt::NoFocus);
        auto *search_log_menu = new QMenu(search_log_button);
        search_log_menu->addAction(QStringLiteral("open search log"), this, [this]() {
            main->notify(QStringLiteral("gallery_seed_log"), id);
        });
        search_log_menu->addAction(QStringLiteral("clear finished"), this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        search_log_button->setMenu(search_log_menu);
        search_log_layout->addWidget(search_log_button, 0);
        search_layout->addWidget(search_log_row);

        auto *search_net_frame = new QFrame(search_box);
        search_net_frame->setFrameStyle(QFrame::Box | QFrame::Raised);
        auto *search_net_layout = new QVBoxLayout(search_net_frame);
        search_net_layout->setContentsMargins(6, 4, 6, 4);
        search_net_layout->setSpacing(2);

        auto *search_net_text_row = new QWidget(search_net_frame);
        auto *search_net_text_layout = new QHBoxLayout(search_net_text_row);
        search_net_text_layout->setContentsMargins(0, 0, 0, 0);
        search_net_text_layout->setSpacing(2);
        auto *search_net_left = new QLabel(search_net_text_row);
        search_net_left->setTextFormat(Qt::PlainText);
        search_net_text_layout->addWidget(search_net_left, 1);
        auto *search_net_right = new QLabel(search_net_text_row);
        search_net_right->setTextFormat(Qt::PlainText);
        search_net_text_layout->addWidget(search_net_right, 0);
        search_net_layout->addWidget(search_net_text_row);

        auto *search_net_gauge_row = new QWidget(search_net_frame);
        auto *search_net_gauge_layout = new QHBoxLayout(search_net_gauge_row);
        search_net_gauge_layout->setContentsMargins(0, 0, 0, 0);
        search_net_gauge_layout->setSpacing(2);
        search_network_progress = new QProgressBar(search_net_gauge_row);
        search_network_progress->setRange(0, 100);
        search_network_progress->setValue(0);
        search_network_progress->setFormat(QStringLiteral("%p%"));
        search_network_progress->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        search_network_progress->setFixedHeight(20);
        search_net_gauge_layout->addWidget(search_network_progress, 1);
        search_cancel =
            row_icon(search_net_gauge_row, QStringLiteral("stop"), QStringLiteral("cancel"));
        search_net_gauge_layout->addWidget(search_cancel, 0);
        search_cog = row_icon(
            search_net_gauge_row, QStringLiteral("gear"), QStringLiteral("network job options"));
        search_net_gauge_layout->addWidget(search_cog, 0);
        search_net_layout->addWidget(search_net_gauge_row);

        search_layout->addWidget(search_net_frame);
        main_layout->addWidget(search_box);

        auto *url_input_row = new QWidget(main_content);
        auto *url_input_layout = new QHBoxLayout(url_input_row);
        url_input_layout->setContentsMargins(0, 0, 0, 0);
        url_input_layout->setSpacing(2);
        url_input = new QLineEdit(url_input_row);
        url_input->setPlaceholderText(
            QStringLiteral("any url hydrus recognises, or a raw file url"));
        url_input_layout->addWidget(url_input, 1);
        url_paste = row_icon(
            url_input_row,
            QStringLiteral("paste"),
            QStringLiteral("Paste multiple inputs from the clipboard. Assumes the texts are "
                           "newline-separated."));
        url_input_layout->addWidget(url_paste, 0);
        main_layout->addWidget(url_input_row);

        connect(url_input, &QLineEdit::returnPressed, this, [this]() {
            if (!url_input->text().trimmed().isEmpty()) {
                start();
                url_input->clear();
            }
        });
        connect(url_paste, &QToolButton::clicked, this, [this]() {
            const QString text = QApplication::clipboard()->text().trimmed();
            if (!text.isEmpty()) {
                url_input->setText(text);
                start();
                url_input->clear();
            }
        });
    } else if (watcher) {
        // Python's SidebarImporterMultipleWatcher: status labels, the watchers
        // table with its action icons, the url input and the options buttons.
        // The imports/search panels live in "highlighted watcher" below.
        status_top = new QLabel(main_content);
        status_top->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_top);

        status_bottom = new QLabel(main_content);
        status_bottom->setTextFormat(Qt::PlainText);
        main_layout->addWidget(status_bottom);

        watcher_table = new QTableWidget(main_content);
        watcher_table->setColumnCount(6);
        watcher_table->setHorizontalHeaderLabels({QStringLiteral("subject"),
                                                  QStringLiteral("files status"),
                                                  QStringLiteral("checking status"),
                                                  QStringLiteral("status"),
                                                  QStringLiteral("items"),
                                                  QStringLiteral("added")});
        watcher_table->verticalHeader()->setVisible(false);
        watcher_table->setSelectionBehavior(QAbstractItemView::SelectRows);
        watcher_table->setSelectionMode(QAbstractItemView::SingleSelection);
        watcher_table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        watcher_table->setHorizontalScrollMode(QAbstractItemView::ScrollPerPixel);
        watcher_table->setFixedHeight(112);
        watcher_table->setTextElideMode(Qt::ElideRight);
        watcher_table->horizontalHeader()->setTextElideMode(Qt::ElideRight);
        watcher_table->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        watcher_table->horizontalHeader()->setFixedHeight(24);

        const int watcher_widths[] = {80, 38, 38, 48, 38, 46};

        for (int index = 0; index < 6; ++index) {
            watcher_table->setColumnWidth(index, watcher_widths[index]);
            watcher_table->horizontalHeader()->setSectionResizeMode(index, QHeaderView::Fixed);
        }

        main_layout->addWidget(watcher_table);

        auto *watcher_icons = new QWidget(main_content);
        auto *watcher_icons_layout = new QHBoxLayout(watcher_icons);
        watcher_icons_layout->setContentsMargins(0, 0, 0, 0);
        watcher_icons_layout->setSpacing(2);
        watcher_icons_layout->addStretch(1);

        watcher_highlight =
            row_icon(watcher_icons, QStringLiteral("highlight"), QStringLiteral("highlight"));
        watcher_icons_layout->addWidget(watcher_highlight, 0);
        watcher_clear = row_icon(
            watcher_icons, QStringLiteral("unhighlight"), QStringLiteral("clear highlight"));
        watcher_icons_layout->addWidget(watcher_clear, 0);
        watcher_files_pause = row_icon(
            watcher_icons, QStringLiteral("pause"), QStringLiteral("pause/play files"));
        watcher_icons_layout->addWidget(watcher_files_pause, 0);
        watcher_check_pause = row_icon(
            watcher_icons, QStringLiteral("pause"), QStringLiteral("pause/play checking"));
        watcher_icons_layout->addWidget(watcher_check_pause, 0);

        auto *check_now = new QPushButton(QStringLiteral("check now"), watcher_icons);
        check_now->setMinimumHeight(22);
        watcher_icons_layout->addWidget(check_now, 0);

        watcher_retry =
            row_icon(watcher_icons, QStringLiteral("retry"), QStringLiteral("retry commands"));
        watcher_icons_layout->addWidget(watcher_retry, 0);
        watcher_trash = row_icon(
            watcher_icons, QStringLiteral("trash"), QStringLiteral("remove selected"));
        watcher_icons_layout->addWidget(watcher_trash, 0);
        main_layout->addWidget(watcher_icons);

        auto *watcher_url_row = new QWidget(main_content);
        auto *watcher_url_layout = new QHBoxLayout(watcher_url_row);
        watcher_url_layout->setContentsMargins(0, 0, 0, 0);
        watcher_url_layout->setSpacing(2);

        watcher_url = new QLineEdit(watcher_url_row);
        watcher_url->setPlaceholderText(QStringLiteral("watcher url"));
        watcher_url_layout->addWidget(watcher_url, 1);
        watcher_paste = row_icon(
            watcher_url_row,
            QStringLiteral("paste"),
            QStringLiteral("Paste multiple inputs from the clipboard. Assumes the texts are "
                           "newline-separated."));
        watcher_url_layout->addWidget(watcher_paste, 0);
        main_layout->addWidget(watcher_url_row);

        checker_options = new QPushButton(QStringLiteral("checker options"), main_content);
        checker_options->setMinimumHeight(22);
        main_layout->addWidget(checker_options);

        connect(check_now, &QPushButton::clicked, this, [this]() {
            if (!main->building) {
                start();
            }
        });
        connect(watcher_highlight, &QToolButton::clicked, this, [this]() {
            if (watcher_table->rowCount() > 0) {
                watcher_table->selectRow(0);
            }
        });
        connect(watcher_clear, &QToolButton::clicked, this, [this]() {
            watcher_table->clearSelection();
        });
        connect(watcher_files_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(watcher_check_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        connect(watcher_retry, &QToolButton::clicked, this, [this]() {
            auto *menu = new QMenu(this);
            menu->setAttribute(Qt::WA_DeleteOnClose);
            menu->addAction(QStringLiteral("retry failed"), this, [this]() {
                main->notify(QStringLiteral("downloader_retry"), id);
            });
            menu->popup(
                watcher_retry->mapToGlobal(QPoint(0, watcher_retry->height())));
        });
        connect(watcher_trash, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("watcher_remove"), id);
        });
        connect(watcher_url, &QLineEdit::returnPressed, this, [this]() {
            const QString text = watcher_url->text().trimmed();

            if (!text.isEmpty()) {
                main->notify(QStringLiteral("watcher_add_url"), id, text);
                watcher_url->clear();
            }
        });
        connect(watcher_paste, &QToolButton::clicked, this, [this]() {
            const QString text = QApplication::clipboard()->text().trimmed();

            if (!text.isEmpty()) {
                const QStringList lines = text.split(QRegularExpression(QStringLiteral("[\r\n]+")),
                                                     Qt::SkipEmptyParts);

                if (!lines.isEmpty()) {
                    main->notify(QStringLiteral("watcher_add_url"), id, lines.first().trimmed());
                }
            }
        });
        connect(checker_options, &QPushButton::clicked, this, [this]() {
            main->notify(QStringLiteral("watcher_checker_options"), id);
        });
    }

    import_options_button = new QPushButton(
        QStringLiteral("import options (%1)")
            .arg(state.value(QStringLiteral("import_options_summary")).toString()),
        main_content);
    import_options_button->setMinimumHeight(22);
    main_layout->addWidget(import_options_button);

    connect(import_options_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("downloader_import_options"), id);
    });

    if (urls) {
        auto *options_menu = new QMenu(import_options_button);
        options_menu->addAction(
            QStringLiteral("edit import options..."), this, [this]() {
                main->notify(QStringLiteral("downloader_import_options"), id);
            });
        import_options_button->setMenu(options_menu);
    }
    if (start_button != nullptr) {
        connect(start_button, &QPushButton::clicked, this, [this]() { start(); });
    }

    if (auto_button != nullptr) {
        connect(auto_button, &QPushButton::clicked, this, [this]() {
            main->notify(QStringLiteral("watcher_auto"), id);
        });
    }

    if (query != nullptr) {
        connect(file_limit, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int value) {
            apply_file_limit(value, no_limit->isChecked());
        });
        connect(no_limit, &QCheckBox::toggled, this, [this](bool checked) {
            apply_file_limit(file_limit->value(), checked);
        });
    }

    if (gallery || watcher) {
        QVBoxLayout *highlight_layout = nullptr;
        QPushButton *highlight_expand = nullptr;
        highlight_box = make_static_box(
            gallery ? QStringLiteral("highlighted query")
                    : QStringLiteral("highlighted watcher"),
            sidebar_content,
            &highlight_layout,
            &highlight_expand);
        highlight_content = highlight_layout->parentWidget();
        sidebar_layout->addWidget(highlight_box);

        connect(highlight_expand, &QPushButton::clicked, this, [this, highlight_expand]() {
            const bool show = !highlight_content->isVisible();
            highlight_content->setVisible(show);
            highlight_expand->setIcon(
                style()->standardIcon(show ? QStyle::SP_ArrowUp : QStyle::SP_ArrowDown));
        });

        highlight_query = new QLineEdit(highlight_content);
        highlight_query->setReadOnly(true);
        highlight_query->setFixedHeight(22);

        if (watcher) {
            highlight_subject = new QLabel(highlight_content);
            highlight_subject->setTextFormat(Qt::PlainText);
            QPalette subject_palette = highlight_subject->palette();
            subject_palette.setColor(QPalette::WindowText, QColor(110, 110, 110));
            highlight_subject->setPalette(subject_palette);
            highlight_layout->addWidget(highlight_subject);
        }

        highlight_layout->addWidget(highlight_query);

        imports_box = make_status_box(highlight_content,
                                      highlight_layout,
                                      QStringLiteral("imports"),
                                      &imports_status,
                                      &file_seed_status,
                                      &files_pause,
                                      &file_log_button,
                                      &file_progress,
                                      &file_percent,
                                      &file_network_progress,
                                      &file_network_percent,
                                      &file_cog,
                                      QStringLiteral("pause/play files"),
                                      QStringLiteral("file"),
                                      QStringLiteral("file_seed_cache"));

        search_box = make_status_box(highlight_content,
                                     highlight_layout,
                                     QStringLiteral("search"),
                                     &search_status,
                                     &search_seed_status,
                                     &search_pause,
                                     &search_log_button,
                                     &search_progress,
                                     &search_percent,
                                     &search_network_progress,
                                     &search_network_percent,
                                     &search_cog,
                                     QStringLiteral("pause/play search"),
                                     gallery ? QStringLiteral("search")
                                             : QStringLiteral("check"),
                                     QStringLiteral("gallery_seed_log"));

        if (gallery) {
            highlight_limit_row = new QWidget(highlight_content);
        auto *highlight_limit_layout = new QHBoxLayout(highlight_limit_row);
        highlight_limit_layout->setContentsMargins(0, 0, 0, 0);
        highlight_limit_layout->setSpacing(4);
        highlight_limit_layout->addWidget(
            new QLabel(QStringLiteral("stop after this many files:"), highlight_limit_row), 0);

        highlight_file_limit = new QSpinBox(highlight_limit_row);
        highlight_file_limit->setRange(1, 10000000);
        highlight_file_limit->setValue(2000);
        highlight_file_limit->setFixedHeight(20);
        highlight_limit_layout->addWidget(highlight_file_limit, 1);

        highlight_no_limit = new QCheckBox(QStringLiteral("no limit"), highlight_limit_row);
        highlight_limit_layout->addWidget(highlight_no_limit, 0);
        highlight_layout->addWidget(highlight_limit_row);

        connect(
            highlight_file_limit,
            QOverload<int>::of(&QSpinBox::valueChanged),
            this,
            [this](int value) { apply_file_limit(value, highlight_no_limit->isChecked()); });
        connect(highlight_no_limit, &QCheckBox::toggled, this, [this](bool checked) {
            apply_file_limit(highlight_file_limit->value(), checked);
        });

        highlight_import_options = new QPushButton(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()),
            highlight_content);
        highlight_import_options->setMinimumHeight(22);
        highlight_layout->addWidget(highlight_import_options);

        connect(highlight_import_options, &QPushButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_import_options"), id);
        });
        }

    } else if (simple) {
        imports_box = make_status_box(main_content,
                                      main_layout,
                                      QStringLiteral("imports"),
                                      &imports_status,
                                      &file_seed_status,
                                      &files_pause,
                                      &file_log_button,
                                      &file_progress,
                                      &file_percent,
                                      &file_network_progress,
                                      &file_network_percent,
                                      &file_cog,
                                      QStringLiteral("pause/play files"),
                                      QStringLiteral("file"),
                                      QStringLiteral("file_seed_cache"));

        search_box = make_status_box(main_content,
                                     main_layout,
                                     simple ? QStringLiteral("parsing") : QStringLiteral("search"),
                                     &search_status,
                                     &search_seed_status,
                                     &search_pause,
                                     &search_log_button,
                                     &search_progress,
                                     &search_percent,
                                     &search_network_progress,
                                     &search_network_percent,
                                     &search_cog,
                                     simple ? QStringLiteral("pause/play queue")
                                            : QStringLiteral("pause/play search"),
                                     simple ? QStringLiteral("parsing") : QStringLiteral("search"),
                                     QStringLiteral("gallery_seed_log"));

        main_layout->removeWidget(imports_box);
        main_layout->removeWidget(search_box);
        main_layout->insertWidget(0, imports_box);
        main_layout->insertWidget(1, search_box);

        if (simple) {
            // Python's SidebarImporterSimpleDownloader puts the pending page
            // queue, the page url input and the formula picker inside "parsing".
            auto *parsing_layout = qobject_cast<QVBoxLayout *>(search_box->layout());

            if (parsing_layout != nullptr) {
                auto *jobs_row = new QWidget(search_box);
                auto *jobs_layout = new QHBoxLayout(jobs_row);
                jobs_layout->setContentsMargins(0, 0, 0, 0);
                jobs_layout->setSpacing(2);

                simple_jobs = new QListWidget(jobs_row);
                simple_jobs->setSelectionMode(QAbstractItemView::ExtendedSelection);
                simple_jobs->setMinimumHeight(8 * 24);
                simple_jobs->setMaximumHeight(8 * 24);
                jobs_layout->addWidget(simple_jobs, 1);

                auto *jobs_buttons = new QWidget(jobs_row);
                auto *jobs_buttons_layout = new QVBoxLayout(jobs_buttons);
                jobs_buttons_layout->setContentsMargins(0, 0, 0, 0);
                jobs_buttons_layout->setSpacing(2);

                auto *job_up = new QPushButton(QString(QChar(0x2191)), jobs_buttons);
                auto *job_delete = new QPushButton(QStringLiteral("X"), jobs_buttons);
                auto *job_down = new QPushButton(QString(QChar(0x2193)), jobs_buttons);

                for (QPushButton *button : {job_up, job_delete, job_down}) {
                    button->setFixedWidth(75);
                    button->setMinimumHeight(24);
                    jobs_buttons_layout->addWidget(button, 0, Qt::AlignHCenter);
                }

                jobs_buttons_layout->addStretch(1);
                jobs_layout->addWidget(jobs_buttons, 0);
                parsing_layout->addWidget(jobs_row);

                const auto push_jobs = [this](bool remove_missing) {
                    QJsonArray urls;

                    for (int row = 0; row < simple_jobs->count(); ++row) {
                        urls.append(
                            simple_jobs->item(row)->data(Qt::UserRole).toString());
                    }

                    QJsonObject payload;
                    payload.insert(QStringLiteral("urls"), urls);
                    payload.insert(QStringLiteral("remove_missing"), remove_missing);
                    main->notify(
                        QStringLiteral("downloader_jobs_set"), id, json_payload(payload));
                };

                connect(job_up, &QPushButton::clicked, this, [this, push_jobs]() {
                    const int row = simple_jobs->currentRow();

                    if (row > 0) {
                        QListWidgetItem *item = simple_jobs->takeItem(row);
                        simple_jobs->insertItem(row - 1, item);
                        simple_jobs->setCurrentRow(row - 1);
                        push_jobs(false);
                    }
                });
                connect(job_down, &QPushButton::clicked, this, [this, push_jobs]() {
                    const int row = simple_jobs->currentRow();

                    if (row >= 0 && row < simple_jobs->count() - 1) {
                        QListWidgetItem *item = simple_jobs->takeItem(row);
                        simple_jobs->insertItem(row + 1, item);
                        simple_jobs->setCurrentRow(row + 1);
                        push_jobs(false);
                    }
                });
                connect(job_delete, &QPushButton::clicked, this, [this, push_jobs]() {
                    const QList<QListWidgetItem *> selected = simple_jobs->selectedItems();

                    for (QListWidgetItem *item : selected) {
                        delete simple_jobs->takeItem(simple_jobs->row(item));
                    }

                    push_jobs(true);
                });

                auto *page_url_row = new QWidget(search_box);
                auto *page_url_layout = new QHBoxLayout(page_url_row);
                page_url_layout->setContentsMargins(0, 0, 0, 0);
                page_url_layout->setSpacing(2);
                simple_url = new QLineEdit(page_url_row);
                simple_url->setPlaceholderText(
                    QStringLiteral("url to be parsed by the selected formula"));
                page_url_layout->addWidget(simple_url, 1);
                simple_paste = row_icon(
                    page_url_row,
                    QStringLiteral("paste"),
                    QStringLiteral("Paste multiple inputs from the clipboard. Assumes the texts "
                                   "are newline-separated."));
                page_url_layout->addWidget(simple_paste, 0);
                parsing_layout->addWidget(page_url_row);

                auto *formula_row = new QWidget(search_box);
                auto *formula_layout = new QHBoxLayout(formula_row);
                formula_layout->setContentsMargins(0, 0, 0, 0);
                formula_layout->setSpacing(2);

                simple_formula = new QComboBox(formula_row);
                simple_formula->setMinimumWidth(
                    fontMetrics().horizontalAdvance(QStringLiteral("0000000000")));
                simple_formula->addItem(
                    QStringLiteral("all files linked by images in page"));
                simple_formula->addItem(QStringLiteral("all images embedded in page"));
                formula_layout->addWidget(simple_formula, 1);

                simple_formula_cog =
                    row_icon(formula_row, QStringLiteral("gear"), QStringLiteral("formula options"));
                formula_layout->addWidget(simple_formula_cog, 0);
                parsing_layout->addWidget(formula_row);

                connect(simple_url, &QLineEdit::returnPressed, this, [this]() {
                    if (!simple_url->text().trimmed().isEmpty()) {
                        start();
                        simple_url->clear();
                    }
                });
                connect(simple_formula,
                        &QComboBox::currentTextChanged,
                        this,
                        [this](const QString &text) {
                            if (!main->building) {
                                main->notify(QStringLiteral("downloader_set_formula"), id, text);
                            }
                        });
                connect(simple_paste, &QToolButton::clicked, this, [this]() {
                    const QString text = QApplication::clipboard()->text().trimmed();
                    if (!text.isEmpty()) {
                        simple_url->setText(text);
                        start();
                        simple_url->clear();
                    }
                });
                connect(simple_formula_cog, &QToolButton::clicked, this, [this]() {
                    auto *menu = new QMenu(this);
                    menu->setAttribute(Qt::WA_DeleteOnClose);
                    menu->addAction(QStringLiteral("edit formulae"), this, [this]() {
                        main->notify(QStringLiteral("options"), id);
                    });
                    menu->popup(
                        simple_formula_cog->mapToGlobal(QPoint(0, simple_formula_cog->height())));
                });
            }
        }
    }

    connect(files_pause, &QToolButton::clicked, this, [this]() {
        main->notify(QStringLiteral("downloader_pause"), id);
    });
    if (search_pause != nullptr) {
        connect(search_pause, &QToolButton::clicked, this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
    }

    const auto network_cog_menu = [this](QToolButton *button) {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("pause files"), this, [this]() {
            main->notify(QStringLiteral("downloader_pause"), id);
        });
        menu->addAction(QStringLiteral("clear finished"), this, [this]() {
            main->notify(QStringLiteral("downloader_clear"), id);
        });
        menu->popup(button->mapToGlobal(QPoint(0, button->height())));
    };

    connect(file_cog, &QToolButton::clicked, this, [network_cog_menu, this]() {
        network_cog_menu(file_cog);
    });
    connect(search_cog, &QToolButton::clicked, this, [network_cog_menu, this]() {
        network_cog_menu(search_cog);
    });

    QVBoxLayout *tags_layout = nullptr;
    auto *tags_box = make_static_box(
        QStringLiteral("selection tags"), sidebar_content, &tags_layout);
    selection_tags_box = tags_box;
    tags_box->setMinimumHeight(180);
    sidebar_layout->addWidget(tags_box, 1);

    tag_sort_type = new QPushButton(QStringLiteral("sort by tag"), tags_box);
    tag_sort_type->setMinimumHeight(24);
    tag_sort_type->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);

    tag_sort_order = new QPushButton(QStringLiteral("a-z"), tags_box);
    tag_sort_order->setMinimumHeight(24);
    tag_sort_order->setFixedWidth(75);

    tag_sort_namespace = new QPushButton(QStringLiteral("namespace (user)"), tags_box);
    tag_sort_namespace->setMinimumHeight(24);
    tag_sort_namespace->setFixedWidth(101);

    auto *tag_sort_row = new QWidget(tags_box);
    auto *tag_sort_row_layout = new QHBoxLayout(tag_sort_row);
    tag_sort_row_layout->setContentsMargins(0, 0, 0, 0);
    tag_sort_row_layout->setSpacing(2);
    tag_sort_row_layout->addWidget(tag_sort_type, 1);
    tag_sort_row_layout->addWidget(tag_sort_order, 0);
    tag_sort_row_layout->addWidget(tag_sort_namespace, 0);
    tags_layout->addWidget(tag_sort_row);

    selection_tags = new QListWidget(tags_box);
    selection_tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
    selection_tags->setMinimumHeight(120);
    tags_layout->addWidget(selection_tags, 1);

    connect(tag_sort_order, &QPushButton::clicked, this, [this]() {
        tag_sort_order->setText(
            tag_sort_order->text() == QStringLiteral("a-z") ? QStringLiteral("z-a")
                                                            : QStringLiteral("a-z"));
        selection_tags->sortItems(
            tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                            : Qt::DescendingOrder);
    });

    fill_sites(state.value(QStringLiteral("sites")).toArray());

    // Make the scroll area aware of the real content minimum; otherwise Qt
    // squeezes the selection tags box below the viewport instead of scrolling.
    sidebar_content->setMinimumHeight(sidebar_layout->minimumSize().height());

    update_state(state);
}
const QString &HydrusDownloaderPage::page_id() const {
    return id;
}

void HydrusDownloaderPage::start() {
    const bool gallery = kind == QStringLiteral("gallery");

    if (gallery && query != nullptr && query->text().trimmed().isEmpty()
        && (url == nullptr || url->text().isEmpty())) {
        query->setFocus();
        return;
    }

    QJsonObject payload;

    if (input != nullptr) {
        payload.insert(QStringLiteral("input"), input->toPlainText());
    }

    if (url_input != nullptr) {
        payload.insert(QStringLiteral("input"), url_input->text());
    }

    if (simple_url != nullptr) {
        payload.insert(QStringLiteral("input"), simple_url->text());
    }

    if (simple_formula != nullptr) {
        payload.insert(QStringLiteral("formula"), simple_formula->currentText());
    }

    if (url != nullptr) {
        payload.insert(QStringLiteral("url"), url->text());
    }

    if (query != nullptr) {
        payload.insert(QStringLiteral("query"), query->text());
    }

    if (site != nullptr) {
        payload.insert(QStringLiteral("site"), site->currentData().toString());
    }

    if (subject != nullptr) {
        payload.insert(QStringLiteral("subject"), subject->text());
    }

    if (period != nullptr) {
        payload.insert(QStringLiteral("period"), period->value());
    }

    payload.insert(
        QStringLiteral("auto"),
        auto_button != nullptr && auto_button->text().endsWith(QStringLiteral("on")));

    const bool unlimited = no_limit != nullptr && no_limit->isChecked();
    const int limit = file_limit != nullptr ? file_limit->value() : 2000;
    payload.insert(QStringLiteral("no_limit"), unlimited);
    payload.insert(QStringLiteral("file_limit"), unlimited ? 0 : limit);

    if (gallery && url != nullptr && url->text().isEmpty() && site != nullptr
        && site->currentIndex() > 0) {
        payload.insert(
            QStringLiteral("url"),
            site->itemData(site->currentIndex(), Qt::UserRole + 1).toString());
    }

    const QString action =
        kind == QStringLiteral("watcher") ? QStringLiteral("watcher_check")
                                          : QStringLiteral("downloader_start");
    main->notify(action, id, json_payload(payload));
}

void HydrusDownloaderPage::fill_sites(const QJsonArray &value) {
    sites = value;

    if (site == nullptr) {
        return;
    }

    bool same = site->count() == value.size() + 1;

    for (int index = 0; same && index < value.size(); ++index) {
        same = site->itemText(index + 1)
               == value.at(index).toObject().value(QStringLiteral("title")).toString();
    }

    if (same) {
        return;
    }

    const QString current = site->currentData().toString();
    const QSignalBlocker blocker(site);
    site->clear();
    site->addItem(QStringLiteral("no downloader set"), QString());

    for (const QJsonValue &entry : value) {
        const QJsonObject parser = entry.toObject();
        site->addItem(
            parser.value(QStringLiteral("title")).toString(),
            parser.value(QStringLiteral("id")).toString());
        site->setItemData(
            site->count() - 1,
            parser.value(QStringLiteral("template")).toString(),
            Qt::UserRole + 1);
    }

    const int index = site->findData(current);
    site->setCurrentIndex(index >= 0 ? index : 0);
}

void HydrusDownloaderPage::apply_file_limit(int value, bool unlimited) {
    const int clamped = qMax(1, value);
    file_limit_value = unlimited ? 0 : clamped;
    no_limit_value = unlimited;

    if (file_limit != nullptr) {
        const QSignalBlocker blocker(file_limit);
        file_limit->setValue(clamped);
        file_limit->setEnabled(!unlimited);
    }

    if (no_limit != nullptr) {
        const QSignalBlocker blocker(no_limit);
        no_limit->setChecked(unlimited);
    }

    if (highlight_file_limit != nullptr) {
        const QSignalBlocker blocker(highlight_file_limit);
        highlight_file_limit->setValue(clamped);
        highlight_file_limit->setEnabled(!unlimited);
    }

    if (highlight_no_limit != nullptr) {
        const QSignalBlocker blocker(highlight_no_limit);
        highlight_no_limit->setChecked(unlimited);
    }
}

void HydrusDownloaderPage::update_state(const QJsonObject &state) {
    kind = state.value(QStringLiteral("kind")).toString();

    const bool gallery = kind == QStringLiteral("gallery");
    const bool watcher = kind == QStringLiteral("watcher");
    const bool urls = kind == QStringLiteral("urls");
    const bool simple = kind == QStringLiteral("simple");

    const bool running = state.value(QStringLiteral("running")).toBool();
    const bool paused = state.value(QStringLiteral("paused")).toBool();
    const bool auto_watch = state.value(QStringLiteral("auto_watch")).toBool();
    const int imported = state.value(QStringLiteral("imported")).toInt();
    const int failed = state.value(QStringLiteral("failed")).toInt();
    const int queue_count = state.value(QStringLiteral("queue_count")).toInt();
    const int queue_done = state.value(QStringLiteral("queue_done")).toInt();
    const int seed_total = state.value(QStringLiteral("gallery_seed_count")).toInt();
    const int seed_done = state.value(QStringLiteral("gallery_seed_done")).toInt();
    const int seed_success = state.value(QStringLiteral("gallery_seed_success")).toInt();
    const int seed_failed = state.value(QStringLiteral("gallery_seed_failed")).toInt();
    const QString status = state.value(QStringLiteral("status")).toString();

    fill_sites(state.value(QStringLiteral("sites")).toArray());

    if (input_row != nullptr) {
        input_row->setVisible(urls || simple);
    }

    if (query_row != nullptr) {
        query_row->setVisible(gallery);
    }

    if (url_row != nullptr) {
        url_row->setVisible(watcher);
    }

    if (site_row != nullptr) {
        site_row->setVisible(watcher || simple);
    }

    if (subject_row != nullptr) {
        subject_row->setVisible(watcher);
    }

    if (period_row != nullptr) {
        period_row->setVisible(watcher);
    }

    if (file_limit_row != nullptr) {
        file_limit_row->setVisible(gallery);
    }

    if (gug_button != nullptr) {
        gug_button->setVisible(gallery);
    }

    if (highlight_box != nullptr) {
        highlight_box->setVisible(gallery || watcher);
    }

    if (start_button != nullptr) {
        start_button->setVisible(!gallery);
        start_button->setEnabled(!running);
        start_button->setText(watcher ? QStringLiteral("check now") : QStringLiteral("start"));
    }

    if (auto_button != nullptr) {
        auto_button->setVisible(watcher);
        auto_button->setText(
            auto_watch ? QStringLiteral("automatic checks: on")
                       : QStringLiteral("automatic checks: off"));
    }

    if (input != nullptr && !input->hasFocus()) {
        const QString value = state.value(QStringLiteral("input")).toString();

        if (input->toPlainText() != value) {
            input->setPlainText(value);
        }
    }

    if (query != nullptr && !query->hasFocus()) {
        update_line_edit(query, state.value(QStringLiteral("query")).toString());
    }

    if (url != nullptr) {
        if (gallery) {
            if (url->text().isEmpty()) {
                const QString stored = state.value(QStringLiteral("url")).toString();

                if (!stored.isEmpty()) {
                    url->setText(stored);
                } else if (site != nullptr && site->currentIndex() > 0) {
                    url->setText(
                        site->itemData(site->currentIndex(), Qt::UserRole + 1).toString());
                }
            }
        } else if (!url->hasFocus()) {
            update_line_edit(url, state.value(QStringLiteral("url")).toString());
        }
    }

    if (subject != nullptr && !subject->hasFocus()) {
        update_line_edit(subject, state.value(QStringLiteral("subject")).toString());
    }

    if (period != nullptr && !period->hasFocus()) {
        period->setValue(state.value(QStringLiteral("check_period")).toInt(3600));
    }

    if (site != nullptr) {
        if (!gallery) {
            const int site_index =
                site->findData(state.value(QStringLiteral("site")).toString());

            if (site_index >= 0 && !site->hasFocus()) {
                const QSignalBlocker blocker(site);
                site->setCurrentIndex(site_index);
            }
        } else if (!site_loaded) {
            const int site_index =
                site->findData(state.value(QStringLiteral("site")).toString());

            if (site_index >= 0) {
                const QSignalBlocker blocker(site);
                site->setCurrentIndex(site_index);
            }

            site_loaded = true;
        }
    }

    if (gug_button != nullptr && site != nullptr) {
        gug_button->setText(
            site->currentIndex() > 0 ? site->currentText()
                                     : QStringLiteral("no downloader set"));
    }

    if (highlight_query != nullptr) {
        highlight_query->setText(
            watcher ? state.value(QStringLiteral("url")).toString()
                    : state.value(QStringLiteral("query")).toString());
    }

    if (highlight_subject != nullptr) {
        const QString subject_text = state.value(QStringLiteral("subject")).toString();
        highlight_subject->setVisible(!state.value(QStringLiteral("url")).toString().isEmpty());
        highlight_subject->setText(
            subject_text.isEmpty() ? QStringLiteral("no subject") : subject_text);
    }

    if (status_top != nullptr) {
        if (watcher) {
            const QString page_url = state.value(QStringLiteral("url")).toString();

            if (page_url.isEmpty()) {
                status_top->setText(QStringLiteral("waiting for new watchers"));
                status_bottom->setText(QString());
            } else {
                status_top->setText(
                    QStringLiteral("1 watchers - %1/%2").arg(queue_done).arg(queue_count));

                QString bottom;

                if (imported > 0 || failed > 0) {
                    bottom = QStringLiteral("%1 successful").arg(imported);

                    if (failed > 0) {
                        bottom += QStringLiteral(", %1 failed").arg(failed);
                    }
                }

                status_bottom->setText(bottom);
            }
        } else {
            const bool has_query = !state.value(QStringLiteral("query")).toString().isEmpty()
                                   || !state.value(QStringLiteral("url")).toString().isEmpty();
            status_top->setText(
                has_query ? QStringLiteral("1 query - %1/%2").arg(queue_done).arg(queue_count)
                          : QStringLiteral("waiting for new queries"));
            QString bottom;

            if (has_query) {
                bottom = QStringLiteral("%1 successful").arg(imported);

                if (failed > 0) {
                    bottom += QStringLiteral(", %1 failed").arg(failed);
                }
            }

            status_bottom->setText(bottom);
        }
    }

    if (watcher_table != nullptr) {
        const QString page_url = state.value(QStringLiteral("url")).toString();
        watcher_table->setRowCount(page_url.isEmpty() ? 0 : 1);

        if (!page_url.isEmpty()) {
            QString subject_text = state.value(QStringLiteral("subject")).toString();

            if (subject_text.isEmpty()) {
                subject_text = QStringLiteral("* unknown");
            }

            const QString files_status =
                queue_count > 0
                    ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                    : QString();
            const QString checking_status =
                paused ? QStringLiteral("paused")
                       : (running ? QStringLiteral("checking") : QString());
            QString added;

            const qint64 last_check =
                static_cast<qint64>(state.value(QStringLiteral("last_check")).toDouble());

            if (last_check > 0) {
                const qint64 delta = qMax<qint64>(
                    0, QDateTime::currentSecsSinceEpoch() - last_check);

                if (delta < 60) {
                    added = QStringLiteral("just now");
                } else if (delta < 3600) {
                    added = QStringLiteral("%1 minutes ago").arg(delta / 60);
                } else if (delta < 86400) {
                    added = QStringLiteral("%1 hours ago").arg(delta / 3600);
                } else {
                    added = QStringLiteral("%1 days ago").arg(delta / 86400);
                }
            } else {
                added = QStringLiteral("now");
            }

            const QStringList cells = {subject_text,
                                       files_status,
                                       checking_status,
                                       status,
                                       QString::number(queue_count),
                                       added};

            for (int column = 0; column < cells.size(); ++column) {
                auto *item = watcher_table->item(0, column);

                if (item == nullptr) {
                    item = new QTableWidgetItem();
                    watcher_table->setItem(0, column, item);
                }

                item->setText(cells[column]);
                item->setToolTip(cells[column]);
            }
        }
    }

    if (queue != nullptr) {
        const bool has_query = !state.value(QStringLiteral("query")).toString().isEmpty()
                               || !state.value(QStringLiteral("url")).toString().isEmpty();
        queue->setRowCount(has_query ? 1 : 0);

        if (has_query) {
            QString source = QStringLiteral("unknown downloader");

            if (site != nullptr && site->currentIndex() > 0) {
                source = site->currentText();
            }

            QString added;

            const qint64 last_check =
                static_cast<qint64>(state.value(QStringLiteral("last_check")).toDouble());

            if (last_check > 0) {
                const qint64 delta = qMax<qint64>(
                    0, QDateTime::currentSecsSinceEpoch() - last_check);

                if (delta < 60) {
                    added = QStringLiteral("just now");
                } else if (delta < 3600) {
                    added = QStringLiteral("%1 minutes ago").arg(delta / 60);
                } else if (delta < 86400) {
                    added = QStringLiteral("%1 hours ago").arg(delta / 3600);
                } else {
                    added = QStringLiteral("%1 days ago").arg(delta / 86400);
                }
            }

            const QString files_status =
                queue_count > 0
                    ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                    : QString();
            const QString search_state_text = running
                                                  ? (paused ? QStringLiteral("paused")
                                                            : QStringLiteral("working"))
                                                  : QStringLiteral("ready");
            const QString query_text = state.value(QStringLiteral("query")).toString().isEmpty()
                                           ? state.value(QStringLiteral("url")).toString()
                                           : state.value(QStringLiteral("query")).toString();
            const QStringList cells = {query_text,
                                       source,
                                       files_status,
                                       search_state_text,
                                       status,
                                       QString::number(queue_count),
                                       added};

            for (int column = 0; column < cells.size(); ++column) {
                auto *item = queue->item(0, column);

                if (item == nullptr) {
                    item = new QTableWidgetItem();
                    queue->setItem(0, column, item);
                }

                item->setText(cells[column]);
                item->setToolTip(cells[column]);
            }
        }
    }

    if (imports_status != nullptr) {
        imports_status->setText(
            (imported == 0 && failed == 0 && queue_count == 0)
                ? QString()
                : (failed == 0
                       ? QStringLiteral("%1 successful").arg(imported)
                       : QStringLiteral("%1 successful, %2 failed")
                             .arg(imported)
                             .arg(failed)));
    }

    if (file_seed_status != nullptr) {
        file_seed_status->setText(
            queue_count > 0 ? QStringLiteral("%1/%2").arg(queue_done).arg(queue_count)
                            : QString());
    }

    if (simple_jobs != nullptr) {
        const QJsonArray pending_jobs = state.value(QStringLiteral("pending_jobs")).toArray();
        const QString formula_name = simple_formula != nullptr
                                         ? simple_formula->currentText()
                                         : QStringLiteral("formula");

        simple_jobs->clear();

        for (const QJsonValue &value : pending_jobs) {
            const QString job_url = value.toString();

            if (!job_url.isEmpty()) {
                auto *item = new QListWidgetItem(
                    QStringLiteral("%1: %2").arg(formula_name).arg(job_url), simple_jobs);
                item->setData(Qt::UserRole, job_url);
            }
        }
    }

    if (simple_formula != nullptr && !simple_formula->hasFocus()) {
        const QString formula_name = state.value(QStringLiteral("formula_name")).toString();
        const int formula_index = simple_formula->findText(formula_name);

        if (formula_index >= 0) {
            const QSignalBlocker blocker(simple_formula);
            simple_formula->setCurrentIndex(formula_index);
        }
    }

    if (file_progress != nullptr) {
        file_progress->setValue(
            queue_count > 0 ? queue_done * 100 / queue_count : 0);
    }

    if (files_pause != nullptr) {
        files_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (search_status != nullptr) {
        if (seed_total == 0) {
            search_status->setText(QString());
        } else {
            const int pending = qMax(0, seed_total - seed_success - seed_failed);
            search_status->setText(
                QStringLiteral("%1 successful, %2 pending")
                    .arg(seed_success)
                    .arg(pending));
        }
    }

    if (search_seed_status != nullptr) {
        if (seed_total == 0) {
            search_seed_status->setText(QString());
        } else {
            const int pending = qMax(0, seed_total - seed_success - seed_failed);
            search_seed_status->setText(
                QStringLiteral("%1 successful, %2 pending")
                    .arg(seed_success)
                    .arg(pending));
        }
    }

    if (search_progress != nullptr) {
        search_progress->setValue(
            seed_total > 0 ? seed_done * 100 / seed_total : 0);
    }

    if (search_pause != nullptr) {
        search_pause->setIcon(
            hydrus_small_icon(paused ? QStringLiteral("play") : QStringLiteral("pause")));
    }

    if (import_options_button != nullptr) {
        import_options_button->setText(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()));
    }

    if (highlight_import_options != nullptr) {
        highlight_import_options->setText(
            QStringLiteral("import options (%1)")
                .arg(state.value(QStringLiteral("import_options_summary")).toString()));
    }
}

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
// HydrusDuplicatesPage
// ---------------------------------------------------------------------------

HydrusDuplicatesPage::HydrusDuplicatesPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
    // Python's duplicates page is a sidebar (preparation | filtering |
    // auto-resolution) next to the media area.
    auto *outer = new QHBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    outer->setSpacing(0);

    auto *page_splitter = new QSplitter(Qt::Horizontal, this);

    auto *sidebar_scroll = new QScrollArea(page_splitter);
    sidebar_scroll->setWidgetResizable(true);
    sidebar_scroll->setFrameShape(QFrame::NoFrame);
    sidebar_scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);

    auto *sidebar = new QWidget();
    auto *sidebar_layout = new QVBoxLayout(sidebar);
    sidebar_layout->setContentsMargins(6, 6, 6, 6);
    sidebar_layout->setSpacing(4);
    sidebar_scroll->setWidget(sidebar);

    // Python's sidebars use the smaller 8pt UI font.
    QFont sidebar_font = sidebar->font();

    if (sidebar_font.pointSizeF() > 8.5) {
        sidebar_font.setPointSizeF(sidebar_font.pointSizeF() - 1.0);
    }

    sidebar->setFont(sidebar_font);

    auto *tabs = new QTabWidget(sidebar);
    auto *preparation_tab = new QWidget(tabs);
    auto *filtering_tab = new QWidget(tabs);
    auto *auto_resolution_tab = new QWidget(tabs);
    tabs->addTab(preparation_tab, QStringLiteral("preparation"));
    tabs->addTab(filtering_tab, QStringLiteral("filtering"));
    tabs->addTab(auto_resolution_tab, QStringLiteral("auto-resolution"));
    tabs->setCurrentIndex(1);
    sidebar_layout->addWidget(tabs, 1);

    auto *preparation_layout = new QVBoxLayout(preparation_tab);
    preparation_layout->setContentsMargins(2, 2, 2, 2);
    preparation_layout->setSpacing(4);

    QVBoxLayout *preparation_content = nullptr;
    QPushButton *preparation_expand = nullptr;
    auto *preparation_box = make_static_box(QStringLiteral("potential duplicate pair discovery"),
                                            preparation_tab,
                                            &preparation_content,
                                            &preparation_expand);
    preparation_layout->addWidget(preparation_box);

    auto *preparation_label = new QLabel(
        QStringLiteral("Preparation discovers potential duplicate pairs in the background. "
                       "Use the filtering tab to set up and launch a potential duplicate "
                       "pair search."),
        preparation_box);
    preparation_label->setWordWrap(true);
    preparation_content->addWidget(preparation_label);

    auto *preparation_progress = new QLabel(
        QStringLiteral("Have not yet searched at this distance."),
        preparation_box);
    preparation_progress->setWordWrap(true);
    preparation_content->addWidget(preparation_progress);

    auto *filtering_layout = new QVBoxLayout(filtering_tab);
    filtering_layout->setContentsMargins(2, 2, 2, 2);
    filtering_layout->setSpacing(4);

    QVBoxLayout *search_layout = nullptr;
    QPushButton *search_expand = nullptr;
    auto *search_box = make_static_box(QStringLiteral("potential duplicate pairs search"),
                                       filtering_tab,
                                       &search_layout,
                                       &search_expand);
    filtering_layout->addWidget(search_box);
    QWidget *search_panel = search_layout->parentWidget();

    connect(search_expand, &QPushButton::clicked, this, [search_layout, search_expand]() {
        QWidget *content = search_layout->parentWidget();
        const bool show = !content->isVisible();
        content->setVisible(show);
        search_expand->setIcon(
            search_expand->style()->standardIcon(show ? QStyle::SP_ArrowUp
                                                     : QStyle::SP_ArrowDown));
    });

    dupe_search_type = new QComboBox(search_panel);
    dupe_search_type->addItem(QStringLiteral("at least one file matches the search"), QStringLiteral("one_file"));
    dupe_search_type->addItem(QStringLiteral("both files match the search"), QStringLiteral("both_files"));
    dupe_search_type->addItem(QStringLiteral("the two files match different searches"), QStringLiteral("different_searches"));

    search_query_1 = new QLineEdit(search_panel);
    search_query_1->setPlaceholderText(QStringLiteral("tags and system predicates for the first file"));
    search_location_1 = new QComboBox(search_panel);
    search_location_1->addItem(QStringLiteral("my files"));
    search_location_1->addItem(QStringLiteral("all local files"));
    search_location_1->addItem(QStringLiteral("trash"));

    search_query_2 = new QLineEdit(search_panel);
    search_query_2->setPlaceholderText(QStringLiteral("tags and system predicates for the second file"));
    search_location_2 = new QComboBox(search_panel);
    search_location_2->addItem(QStringLiteral("my files"));
    search_location_2->addItem(QStringLiteral("all local files"));
    search_location_2->addItem(QStringLiteral("trash"));
    search_row_2_label = new QLabel(QStringLiteral("search 2:"), search_panel);

    pixel_dupes = new QComboBox(search_panel);
    pixel_dupes->addItem(QStringLiteral("can be pixel dupes"), QStringLiteral("allowed"));
    pixel_dupes->addItem(QStringLiteral("must be pixel dupes"), QStringLiteral("required"));
    pixel_dupes->addItem(QStringLiteral("must not be pixel dupes"), QStringLiteral("excluded"));

    search_distance = new QSpinBox(search_panel);
    search_distance->setRange(0, 64);
    search_distance->setSingleStep(2);
    search_distance->setToolTip(QStringLiteral(
        "The max hamming distance allowed in the search. Higher distances mean more pairs and more false positives."));

    sort_type = new QComboBox(search_panel);
    sort_type->setSizeAdjustPolicy(QComboBox::AdjustToMinimumContentsLengthWithIcon);
    sort_type->setMinimumContentsLength(14);
    sort_type->addItem(QStringLiteral("sort pairs by: filesize of larger file"), QStringLiteral("max_filesize"));
    sort_type->addItem(QStringLiteral("sort pairs by: filesize of smaller file"), QStringLiteral("min_filesize"));
    sort_type->addItem(QStringLiteral("sort pairs by: similarity (distance/filesize ratio)"), QStringLiteral("similarity"));
    sort_type->addItem(QStringLiteral("sort pairs by: random"), QStringLiteral("random"));

    sort_ascending = new QComboBox(search_panel);
    sort_ascending->setSizeAdjustPolicy(QComboBox::AdjustToMinimumContentsLengthWithIcon);
    sort_ascending->setMinimumContentsLength(10);
    sort_ascending->addItem(QStringLiteral("largest first"), false);
    sort_ascending->addItem(QStringLiteral("smallest first"), true);

    search_summary = new QLabel(search_panel);
    search_summary->setWordWrap(true);

    search_layout->addWidget(dupe_search_type);

    auto *search_row_1 = new QWidget(search_panel);
    auto *search_row_1_layout = new QHBoxLayout(search_row_1);
    search_row_1_layout->setContentsMargins(0, 0, 0, 0);
    search_row_1_layout->setSpacing(2);
    search_row_1_layout->addWidget(search_query_1, 1);
    search_row_1_layout->addWidget(search_location_1, 0);
    search_layout->addWidget(search_row_1);

    auto *search_icons = new QWidget(search_panel);
    auto *search_icons_layout = new QHBoxLayout(search_icons);
    search_icons_layout->setContentsMargins(0, 0, 0, 0);
    search_icons_layout->setSpacing(2);
    search_icons_layout->addStretch(1);
    auto *search_paste = make_icon_button(search_icons,
                                          QStringLiteral("paste"),
                                          QStringLiteral("Paste tags from the clipboard."));
    auto *search_favourite = make_icon_button(search_icons,
                                              QStringLiteral("star"),
                                              QStringLiteral("favourite search"));
    auto *search_clear = make_icon_button(search_icons,
                                          QStringLiteral("clear"),
                                          QStringLiteral("clear the search"));
    auto *search_lock = make_icon_button(search_icons,
                                         QStringLiteral("lock"),
                                         QStringLiteral("synchronise this search"));
    search_icons_layout->addWidget(search_paste);
    search_icons_layout->addWidget(search_favourite);
    search_icons_layout->addWidget(search_clear);
    search_icons_layout->addWidget(search_lock);
    search_layout->addWidget(search_icons);

    connect(search_paste, &QToolButton::clicked, this, [this]() {
        const QString text = QApplication::clipboard()->text().trimmed();

        if (!text.isEmpty()) {
            search_query_1->setText(text);
            notify_search_context();
        }
    });
    connect(search_favourite, &QToolButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("favourite searches are not implemented yet"));
    });
    connect(search_clear, &QToolButton::clicked, this, [this]() {
        search_query_1->clear();
        search_query_2->clear();
        notify_search_context();
    });
    connect(search_lock, &QToolButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("search synchronisation is not implemented yet"));
    });

    search_layout->addWidget(search_row_2_label);

    auto *search_row_2 = new QWidget(search_panel);
    auto *search_row_2_layout = new QHBoxLayout(search_row_2);
    search_row_2_layout->setContentsMargins(0, 0, 0, 0);
    search_row_2_layout->setSpacing(2);
    search_row_2_layout->addWidget(search_query_2, 1);
    search_row_2_layout->addWidget(search_location_2, 0);
    search_layout->addWidget(search_row_2);
    search_row_2->setVisible(false);

    auto *distance_row = new QWidget(search_panel);
    auto *distance_row_layout = new QHBoxLayout(distance_row);
    distance_row_layout->setContentsMargins(0, 0, 0, 0);
    distance_row_layout->setSpacing(4);
    distance_row_layout->addWidget(
        new QLabel(QStringLiteral("maximum search distance of pair: "), distance_row));
    distance_row_layout->addWidget(search_distance, 1);
    search_layout->addWidget(distance_row);

    search_layout->addWidget(pixel_dupes);

    auto *count_row = new QWidget(search_panel);
    auto *count_row_layout = new QHBoxLayout(count_row);
    count_row_layout->setContentsMargins(0, 0, 0, 0);
    count_row_layout->setSpacing(2);
    count_row_layout->addWidget(search_summary, 1);
    auto *count_info = make_icon_button(count_row,
                                        QStringLiteral("help"),
                                        QStringLiteral("what is this?"));
    auto *count_refresh = make_icon_button(count_row,
                                           QStringLiteral("retry"),
                                           QStringLiteral("refresh pair count"));
    auto *count_cog = make_icon_button(count_row,
                                       QStringLiteral("gear"),
                                       QStringLiteral("count options"));
    count_row_layout->addWidget(count_info);
    count_row_layout->addWidget(count_refresh);
    count_row_layout->addWidget(count_cog);
    search_layout->addWidget(count_row);

    connect(count_info, &QToolButton::clicked, this, [this]() {
        main->notify(QStringLiteral("notify"),
                     QString(),
                     QStringLiteral("this counts potential duplicate pairs for the current "
                                    "search"));
    });
    connect(count_refresh, &QToolButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_refresh"));
    });
    connect(count_cog, &QToolButton::clicked, this, [this, count_cog]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("start new potential duplicate pair search panels paused"),
                        this,
                        [this]() {
                            main->notify(QStringLiteral("notify"),
                                         QString(),
                                         QStringLiteral("this option is not implemented yet"));
                        });
        menu->popup(count_cog->mapToGlobal(QPoint(0, count_cog->height())));
    });

    QVBoxLayout *filter_layout = nullptr;
    QPushButton *filter_expand = nullptr;
    auto *filter_box = make_static_box(QStringLiteral("duplicate filter"),
                                       filtering_tab,
                                       &filter_layout,
                                       &filter_expand);
    filtering_layout->addWidget(filter_box);

    auto *merge_button = new QPushButton(
        QStringLiteral("edit default duplicate metadata merge options"), filter_box);
    merge_button->setMinimumHeight(24);
    filter_layout->addWidget(merge_button);
    connect(merge_button, &QPushButton::clicked, this, [this, merge_button]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("edit duplicate metadata merge options for 'this is better'"),
                        this,
                        [this]() { act(QStringLiteral("duplicates_merge_options_dialog")); });
        menu->addAction(QStringLiteral("edit duplicate metadata merge options for 'same quality'"),
                        this,
                        [this]() { act(QStringLiteral("duplicates_merge_options_dialog")); });
        menu->popup(merge_button->mapToGlobal(QPoint(0, merge_button->height())));
    });

    auto *sort_row = new QWidget(filter_box);
    auto *sort_row_layout = new QHBoxLayout(sort_row);
    sort_row_layout->setContentsMargins(0, 0, 0, 0);
    sort_row_layout->setSpacing(2);
    sort_row_layout->addWidget(sort_type, 1);
    sort_row_layout->addWidget(sort_ascending, 0);
    auto *group_mode = new QPushButton(QStringLiteral("mixed pairs"), sort_row);
    group_mode->setMinimumHeight(24);
    group_mode->setFixedWidth(80);
    sort_row_layout->addWidget(group_mode, 0);
    filter_layout->addWidget(sort_row);

    connect(group_mode, &QPushButton::clicked, this, [this, group_mode]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        auto *mixed = menu->addAction(QStringLiteral("mixed pairs"));
        auto *group = menu->addAction(QStringLiteral("group mode"));
        connect(mixed, &QAction::triggered, this, [group_mode]() {
            group_mode->setText(QStringLiteral("mixed pairs"));
        });
        connect(group, &QAction::triggered, this, [this, group_mode]() {
            group_mode->setText(QStringLiteral("group mode"));
            main->notify(QStringLiteral("notify"),
                         QString(),
                         QStringLiteral("group mode is not implemented yet"));
        });
        menu->popup(group_mode->mapToGlobal(QPoint(0, group_mode->height())));
    });

    auto *launch_filter = new QPushButton(QStringLiteral("launch the filter"), filter_box);
    launch_filter->setMinimumHeight(24);
    filter_layout->addWidget(launch_filter);
    connect(launch_filter, &QPushButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_next_batch"));
    });

    QVBoxLayout *quick_layout = nullptr;
    QPushButton *quick_expand = nullptr;
    auto *quick_box = make_static_box(QStringLiteral("quick and dirty processing"),
                                      filtering_tab,
                                      &quick_layout,
                                      &quick_expand);
    filtering_layout->addWidget(quick_box);

    auto *quick_sort_row = new QWidget(quick_box);
    auto *quick_sort_layout = new QHBoxLayout(quick_sort_row);
    quick_sort_layout->setContentsMargins(0, 0, 0, 0);
    quick_sort_layout->setSpacing(2);
    auto *quick_sort_type = new QPushButton(QStringLiteral("sort by file: filesize"),
                                            quick_sort_row);
    quick_sort_type->setMinimumHeight(24);
    auto *quick_sort_order = new QPushButton(QStringLiteral("smallest first"), quick_sort_row);
    quick_sort_order->setMinimumHeight(24);
    quick_sort_order->setFixedWidth(92);
    quick_sort_layout->addWidget(quick_sort_type, 1);
    quick_sort_layout->addWidget(quick_sort_order, 0);
    quick_layout->addWidget(quick_sort_row);

    auto *show_random = new QPushButton(
        QStringLiteral("show some random potential duplicates"), quick_box);
    show_random->setMinimumHeight(24);
    quick_layout->addWidget(show_random);
    auto *set_same_quality = new QPushButton(
        QStringLiteral("set current media as duplicates of the same quality"), quick_box);
    set_same_quality->setMinimumHeight(24);
    quick_layout->addWidget(set_same_quality);
    auto *set_alternates = new QPushButton(
        QStringLiteral("set current media as all related alternates"), quick_box);
    set_alternates->setMinimumHeight(24);
    quick_layout->addWidget(set_alternates);
    auto *set_false_positive = new QPushButton(
        QStringLiteral("set current media as not related/false positive"), quick_box);
    set_false_positive->setMinimumHeight(24);
    quick_layout->addWidget(set_false_positive);

    connect(quick_sort_type, &QPushButton::clicked, this, [this, quick_sort_type]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        menu->addAction(QStringLiteral("sort by file: filesize"), this, [quick_sort_type]() {
            quick_sort_type->setText(QStringLiteral("sort by file: filesize"));
        });
        menu->addAction(QStringLiteral("sort by file: import time"), this, [quick_sort_type]() {
            quick_sort_type->setText(QStringLiteral("sort by file: import time"));
        });
        menu->popup(quick_sort_type->mapToGlobal(QPoint(0, quick_sort_type->height())));
    });
    connect(quick_sort_order, &QPushButton::clicked, this, [quick_sort_order]() {
        quick_sort_order->setText(
            quick_sort_order->text() == QStringLiteral("smallest first")
                ? QStringLiteral("largest first")
                : QStringLiteral("smallest first"));
    });
    connect(show_random, &QPushButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_refresh"));
    });
    connect(set_same_quality, &QPushButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_same"));
    });
    connect(set_alternates, &QPushButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_alternates"));
    });
    connect(set_false_positive, &QPushButton::clicked, this, [this]() {
        act(QStringLiteral("duplicates_false"));
    });

    QVBoxLayout *tags_layout = nullptr;
    QPushButton *tags_expand = nullptr;
    auto *tags_box = make_static_box(QStringLiteral("selection tags"),
                                     filtering_tab,
                                     &tags_layout,
                                     &tags_expand);
    filtering_layout->addWidget(tags_box, 1);

    auto *tag_sort_row = new QWidget(tags_box);
    auto *tag_sort_row_layout = new QHBoxLayout(tag_sort_row);
    tag_sort_row_layout->setContentsMargins(0, 0, 0, 0);
    tag_sort_row_layout->setSpacing(2);
    auto *tag_sort_type = new QPushButton(QStringLiteral("sort by tag"), tag_sort_row);
    auto *tag_sort_order = new QPushButton(QStringLiteral("a-z"), tag_sort_row);
    auto *tag_sort_namespace = new QPushButton(QStringLiteral("namespace (user)"), tag_sort_row);
    tag_sort_type->setMinimumHeight(24);
    tag_sort_order->setMinimumHeight(24);
    tag_sort_order->setFixedWidth(75);
    tag_sort_namespace->setMinimumHeight(24);
    tag_sort_namespace->setFixedWidth(101);
    tag_sort_row_layout->addWidget(tag_sort_type, 1);
    tag_sort_row_layout->addWidget(tag_sort_order, 0);
    tag_sort_row_layout->addWidget(tag_sort_namespace, 0);
    tags_layout->addWidget(tag_sort_row);
    auto *selection_tags = new QListWidget(tags_box);
    selection_tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
    tags_layout->addWidget(selection_tags, 1);

    auto *auto_layout = new QVBoxLayout(auto_resolution_tab);
    auto_layout->setContentsMargins(2, 2, 2, 2);
    auto_layout->setSpacing(4);

    auto *help_button = new QPushButton(QStringLiteral("?"), auto_resolution_tab);
    help_button->setFlat(true);
    auto *help_hbox = new QHBoxLayout();
    help_hbox->addStretch(1);
    help_hbox->addWidget(new QLabel(QStringLiteral("help for this panel -->"), auto_resolution_tab));
    help_hbox->addWidget(help_button);
    auto_layout->addLayout(help_hbox);

    auto *button_hbox = new QHBoxLayout();
    auto *auto_refresh_button = new QPushButton(QStringLiteral("refresh"), auto_resolution_tab);
    auto *cog_button = new QPushButton(QStringLiteral("cog"), auto_resolution_tab);
    button_hbox->addStretch(1);
    button_hbox->addWidget(auto_refresh_button);
    button_hbox->addWidget(cog_button);
    auto_layout->addLayout(button_hbox);

    auto_rules_status = new QLabel(QStringLiteral("initialising..."), auto_resolution_tab);
    auto_rules_status->setWordWrap(true);
    auto_layout->addWidget(auto_rules_status);

    auto_rules_table = new QTableWidget(auto_resolution_tab);
    auto_rules_table->setColumnCount(3);
    auto_rules_table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("progress"),
        QStringLiteral("status"),
    });
    auto_rules_table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    auto_rules_table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    auto_rules_table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    auto_rules_table->setSelectionBehavior(QAbstractItemView::SelectRows);
    auto_rules_table->setSelectionMode(QAbstractItemView::ExtendedSelection);
    auto_rules_table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    auto_rules_table->verticalHeader()->setVisible(false);
    auto_layout->addWidget(auto_rules_table, 1);

    auto *actions_hbox = new QHBoxLayout();
    auto *pause_play = new QPushButton(QStringLiteral("pause/play"), auto_resolution_tab);
    auto *review_actions = new QPushButton(QStringLiteral("review actions"), auto_resolution_tab);
    auto *edit_rules = new QPushButton(QStringLiteral("edit rules"), auto_resolution_tab);
    actions_hbox->addWidget(pause_play);
    actions_hbox->addWidget(review_actions);
    actions_hbox->addWidget(edit_rules);
    auto_layout->addLayout(actions_hbox);

    connect(auto_refresh_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(pause_play, &QPushButton::clicked, this, [this]() {
        // toggle paused state through the rules dialog flow is not available;
        // request a refresh of the rules list instead
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(review_actions, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_review"));
    });
    connect(edit_rules, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(help_button, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("docs"));
    });
    update_auto_resolution_rules(state);

    auto *right = new QWidget(page_splitter);
    auto *layout = new QVBoxLayout(right);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(6);
    page_splitter->addWidget(sidebar_scroll);
    page_splitter->addWidget(right);
    page_splitter->setStretchFactor(0, 0);
    page_splitter->setStretchFactor(1, 1);
    page_splitter->setSizes({440, 872});
    outer->addWidget(page_splitter);

    auto *header = new QHBoxLayout();
    status = new QLabel(this);
    distance = new QLabel(this);
    auto *refresh_button = new QPushButton(QStringLiteral("refresh"), this);
    refresh_button->setObjectName(QStringLiteral("refresh_duplicates"));
    refresh = refresh_button;
    delete_both = new QPushButton(QStringLiteral("delete both"), this);
    rules = new QPushButton(QStringLiteral("auto-resolution..."), this);
    merge_options = new QPushButton(QStringLiteral("merge options..."), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    rules->hide();
    merge_options->hide();
    review->hide();
    batch = new QSpinBox(this);
    batch->setRange(5, 1024);
    batch->setToolTip(QStringLiteral("maximum pairs prepared in one batch"));
    next_batch = new QPushButton(QStringLiteral("next batch"), this);
    status->setMinimumWidth(0);
    status->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    header->addWidget(status, 1);
    header->addWidget(distance);
    header->addWidget(new QLabel(QStringLiteral("batch:"), this));
    header->addWidget(batch);
    header->addWidget(next_batch);
    group_actions = new QPushButton(QStringLiteral("actions..."), this);
    group_actions->setToolTip(QStringLiteral("reset potentials, clear false positives, or dissolve duplicate groups"));
    group_actions->hide();
    header->addWidget(refresh_button);
    header->addWidget(delete_both);
    layout->addLayout(header);

    media_status = new QLabel(right);
    media_status->setWordWrap(true);
    layout->addWidget(media_status);

    grid = new HydrusThumbnailGrid(main, this);
    layout->addWidget(grid, 1);

    auto *bottom = new QHBoxLayout();
    same = new QPushButton(QStringLiteral("they are the same"), this);
    alternates = new QPushButton(QStringLiteral("they are related alternates"), this);
    custom_action = new QPushButton(QStringLiteral("custom action..."), this);
    not_duplicates = new QPushButton(QStringLiteral("not duplicates"), this);
    skip = new QPushButton(QStringLiteral("skip"), this);
    bottom->addStretch(1);
    bottom->addWidget(same);
    bottom->addWidget(alternates);
    bottom->addWidget(custom_action);
    bottom->addWidget(not_duplicates);
    bottom->addWidget(skip);
    bottom->addStretch(1);
    layout->addLayout(bottom);

    sidebar_scroll->setMinimumWidth(440);
    sidebar->setMinimumHeight(sidebar_layout->minimumSize().height());
    page_splitter->setSizes({440, 872});

    connect(same, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_same")); });
    connect(alternates, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_alternates")); });
    connect(custom_action, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_custom_action")); });
    connect(not_duplicates, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_false")); });
    connect(skip, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_skip")); });
    connect(refresh_button, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_refresh")); });
    connect(next_batch, &QPushButton::clicked, this, [this]() { act(QStringLiteral("duplicates_next_batch")); });
    connect(batch, &QSpinBox::valueChanged, this, [this](int value) {
        main->notify(QStringLiteral("duplicates_batch_size"), id, QString::number(value));
    });
    connect(rules, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_rules"));
    });
    connect(merge_options, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_merge_options_dialog"));
    });
    connect(review, &QPushButton::clicked, this, [this]() {
        main->notify(QStringLiteral("duplicates_review"));
    });
    connect(group_actions, &QPushButton::clicked, this, [this]() {
        show_group_actions(QStringLiteral("both"), group_actions->mapToGlobal(QPoint(0, group_actions->height())));
    });
    connect(delete_both, &QPushButton::clicked, this, [this]() {
        if (QMessageBox::question(this, QStringLiteral("delete both"), QStringLiteral("Move both files to the trash?")) == QMessageBox::Yes) {
            act(QStringLiteral("duplicates_delete_both"));
        }
    });

    connect(dupe_search_type, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_location_1, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_location_2, &QComboBox::currentTextChanged, this, [this]() { notify_search_context(); });
    connect(search_query_1, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(search_query_2, &QLineEdit::editingFinished, this, [this]() { notify_search_context(); });
    connect(pixel_dupes, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(search_distance, &QSpinBox::valueChanged, this, [this]() { notify_search_context(); });
    connect(sort_type, &QComboBox::currentIndexChanged, this, [this]() {
        update_search_controls();
        notify_search_context();
    });
    connect(sort_ascending, &QComboBox::currentIndexChanged, this, [this]() { notify_search_context(); });

    update_search_controls();

    update_state(state);

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_CUSTOM_ACTION")) {
        QTimer::singleShot(150, this, [this]() { act(QStringLiteral("duplicates_custom_action")); });
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_ACTIONS_MENU")) {
        QTimer::singleShot(150, this, [this]() {
            show_group_actions(QStringLiteral("both"), mapToGlobal(QPoint(40, 40)));
        });
    }
}

void HydrusDuplicatesPage::notify_search_context() {
    const QString search_type = dupe_search_type->currentData().toString();

    if (search_type != QStringLiteral("different_searches")) {
        const QSignalBlocker blocker(search_location_2);
        search_location_2->setCurrentText(search_location_1->currentText());
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("query_1"), search_query_1->text());
    payload.insert(QStringLiteral("location_1"), search_location_1->currentText());
    payload.insert(QStringLiteral("query_2"), search_query_2->text());
    payload.insert(QStringLiteral("location_2"), search_location_2->currentText());
    payload.insert(QStringLiteral("dupe_search_type"), search_type);
    payload.insert(QStringLiteral("pixel_dupes"), pixel_dupes->currentData().toString());
    payload.insert(QStringLiteral("max_distance"), search_distance->value());
    payload.insert(QStringLiteral("sort_type"), sort_type->currentData().toString());
    payload.insert(QStringLiteral("sort_ascending"), sort_ascending->currentData().toBool());

    main->notify(QStringLiteral("duplicates_search_context"), id, json_payload(payload));
}

void HydrusDuplicatesPage::update_search_controls() {
    const bool different_searches = dupe_search_type->currentData().toString() == QStringLiteral("different_searches");

    search_row_2_label->setVisible(different_searches);
    search_query_2->setVisible(different_searches);
    search_location_2->setVisible(different_searches);

    if (search_query_2->parentWidget() != nullptr) {
        search_query_2->parentWidget()->setVisible(different_searches);
    }

    search_distance->setEnabled(pixel_dupes->currentData().toString() != QStringLiteral("required"));

    const QString sort = sort_type->currentData().toString();
    const bool random = sort == QStringLiteral("random");
    const bool current_asc = sort_ascending->currentData().toBool();

    sort_ascending->setVisible(!random);

    if (random) {
        return;
    }

    QString first;
    QString second;

    if (sort == QStringLiteral("similarity")) {
        first = QStringLiteral("most similar first");
        second = QStringLiteral("least similar first");
    } else {
        first = QStringLiteral("largest first");
        second = QStringLiteral("smallest first");
    }

    const QSignalBlocker blocker(sort_ascending);
    sort_ascending->setItemText(0, first);
    sort_ascending->setItemText(1, second);

    const int index = sort_ascending->findData(current_asc);

    if (index >= 0) {
        sort_ascending->setCurrentIndex(index);
    }
}

const QString &HydrusDuplicatesPage::page_id() const {
    return id;
}

void HydrusDuplicatesPage::show_group_actions(const QString &target, const QPoint &global_pos) {
    QMenu menu(this);
    const QString target_label = target == QStringLiteral("a")
        ? QStringLiteral("file A")
        : target == QStringLiteral("b") ? QStringLiteral("file B") : QStringLiteral("both files");

    auto add = [&menu](const QString &label, const QString &kind, const QString &tooltip) {
        QAction *action = menu.addAction(label);
        action->setData(kind);
        action->setToolTip(tooltip);
    };

    add(QStringLiteral("remove all potentials"),
        QStringLiteral("remove_potentials"),
        QStringLiteral("Stop these files from appearing in the duplicate filter unless new potentials are found."));

    add(QStringLiteral("reset potential search"),
        QStringLiteral("reset_potential_search"),
        QStringLiteral("Search these files for potential duplicates again. Existing pairs are kept."));

    menu.addSeparator();

    add(QStringLiteral("clear all false positives"),
        QStringLiteral("clear_false_positives"),
        QStringLiteral("Remove every false-positive relation for these files and their alternates."));

    add(QStringLiteral("clear internal false positives"),
        QStringLiteral("clear_internal_false_positives"),
        QStringLiteral("Remove false-positive relations between the selected files."));

    menu.addSeparator();

    add(QStringLiteral("remove from duplicate group"),
        QStringLiteral("remove_from_duplicate_group"),
        QStringLiteral("Pull these files out of their duplicate groups."));

    add(QStringLiteral("remove from alternates group"),
        QStringLiteral("remove_from_alternate_group"),
        QStringLiteral("Pull these files out of their alternates groups."));

    add(QStringLiteral("dissolve duplicate group"),
        QStringLiteral("dissolve_duplicate_group"),
        QStringLiteral("Remove the whole duplicate groups and their alternates relations."));

    add(QStringLiteral("dissolve alternates group"),
        QStringLiteral("dissolve_alternate_group"),
        QStringLiteral("Remove every duplicate and alternates relation for the whole alternates groups."));

    QAction *chosen = menu.exec(global_pos);

    if (chosen == nullptr) {
        return;
    }

    const QString kind = chosen->data().toString();

    if (kind.isEmpty()) {
        return;
    }

    const QString warning = QStringLiteral("%1 for %2?\n\nThis can throw away previous duplicate decisions and cannot be undone.")
        .arg(chosen->text(), target_label);

    if (QMessageBox::question(this, QStringLiteral("duplicate action"), warning) != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("kind"), kind);
    payload.insert(QStringLiteral("target"), target);
    main->notify(QStringLiteral("duplicates_group_action"), id, json_payload(payload));
}

void HydrusDuplicatesPage::act(const QString &action) {
    main->notify(action, id);
}

void HydrusDuplicatesPage::update_state(const QJsonObject &state) {
    const int index = state.value(QStringLiteral("index")).toInt();
    const int total = state.value(QStringLiteral("total")).toInt();
    const QString page_status = state.value(QStringLiteral("status")).toString();

    status->setText(total == 0
        ? page_status
        : QStringLiteral("%1  -  %2/%3").arg(page_status).arg(index).arg(total));

    {
        const QSignalBlocker blocker(batch);
        batch->setValue(state.value(QStringLiteral("batch_size")).toInt(100));
    }

    const QJsonObject search = state.value(QStringLiteral("search")).toObject();

    if (!search.isEmpty()) {
        auto set_combo_data = [](QComboBox *combo, const QString &value) {
            const int index = combo->findData(value);

            if (index >= 0) {
                const QSignalBlocker blocker(combo);
                combo->setCurrentIndex(index);
            }
        };

        const QSignalBlocker query_1_blocker(search_query_1);
        const QSignalBlocker query_2_blocker(search_query_2);
        const QSignalBlocker distance_blocker(search_distance);

        const QString query_1 = search.value(QStringLiteral("query_1")).toString();
        const QString query_2 = search.value(QStringLiteral("query_2")).toString();
        search_query_1->setText(query_1.isEmpty() ? QStringLiteral("system:everything")
                                                  : query_1);
        search_query_2->setText(query_2.isEmpty() ? QStringLiteral("system:everything")
                                                  : query_2);
        search_distance->setValue(search.value(QStringLiteral("max_distance")).toInt(4));
        set_combo_data(dupe_search_type, search.value(QStringLiteral("dupe_search_type")).toString());
        set_combo_data(pixel_dupes, search.value(QStringLiteral("pixel_dupes")).toString());

        {
            const QSignalBlocker sort_type_blocker(sort_type);
            const QSignalBlocker sort_asc_blocker(sort_ascending);

            set_combo_data(sort_type, search.value(QStringLiteral("sort_type")).toString());
            const int sort_index = sort_ascending->findData(search.value(QStringLiteral("sort_ascending")).toBool());

            if (sort_index >= 0) {
                sort_ascending->setCurrentIndex(sort_index);
            }
        }

        {
            const QSignalBlocker location_1_blocker(search_location_1);
            const QSignalBlocker location_2_blocker(search_location_2);
            const QString location_1 = search.value(QStringLiteral("location_1")).toString();
            const QString location_2 = search.value(QStringLiteral("location_2")).toString();

            if (!location_1.isEmpty()) {
                search_location_1->setCurrentText(location_1);
            }

            if (!location_2.isEmpty()) {
                search_location_2->setCurrentText(location_2);
            }
        }

        const int matched = search.value(QStringLiteral("matched")).toInt();
        const int searched = state.value(QStringLiteral("searched")).toInt();

        if (searched == 0) {
            search_summary->setText(
                QStringLiteral("no potential pairs in this file domain!"));
        } else {
            search_summary->setText(QStringLiteral("%1 of %2 pairs match  -  %3")
                .arg(matched)
                .arg(searched)
                .arg(search.value(QStringLiteral("summary")).toString()));
        }
    }

    update_search_controls();

    const QJsonValue pair_value = state.value(QStringLiteral("pair"));

    if (!pair_value.isObject()) {
        distance->setText(QString());

        for (QPushButton *button : {same, not_duplicates, skip, delete_both}) {
            button->setEnabled(false);
        }
    } else {
        for (QPushButton *button : {same, not_duplicates, skip, delete_both}) {
            button->setEnabled(true);
        }

        const QJsonObject pair = pair_value.toObject();
        distance->setText(QStringLiteral("distance %1").arg(pair.value(QStringLiteral("distance")).toInt()));
    }

    const QJsonArray media = state.value(QStringLiteral("media")).toArray();

    if (grid != nullptr) {
        grid->set_media(media);
    }

    if (media_status != nullptr) {
        if (media.isEmpty()) {
            media_status->setText(state.value(QStringLiteral("media_empty_status")).toString());
        } else {
            media_status->setText(state.value(QStringLiteral("media_count_status")).toString());
        }
    }

    update_auto_resolution_rules(state);
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

namespace {

class HydrusUrlClassDialog : public QDialog {
public:
    HydrusUrlClassDialog(const QJsonObject &entry, const QJsonArray &types, QWidget *parent = nullptr)
        : QDialog(parent), data(entry) {
        setWindowTitle(QStringLiteral("url class"));
        resize(620, 560);

        auto *layout = new QVBoxLayout(this);
        auto *form = new QFormLayout();

        name = new QLineEdit(this);
        type = new QComboBox(this);
        scheme = new QLineEdit(this);
        netloc = new QLineEdit(this);
        path = new QLineEdit(this);
        parameters = new QPlainTextEdit(this);
        parameters->setMaximumHeight(80);
        parameters->setPlaceholderText(QStringLiteral("one name=value per line"));
        example = new QLineEdit(this);
        match_subdomains = new QCheckBox(QStringLiteral("match subdomains"), this);
        keep_subdomains = new QCheckBox(QStringLiteral("keep the matched subdomain"), this);
        multiple = new QCheckBox(QStringLiteral("can produce multiple files"), this);
        associate = new QCheckBox(QStringLiteral("associate with files"), this);
        associate->setChecked(true);

        for (const QJsonValue &value : types) {
            const QJsonObject item = value.toObject();
            type->addItem(item.value(QStringLiteral("label")).toString(), item.value(QStringLiteral("value")).toInt());
        }

        form->addRow(QStringLiteral("name"), name);
        form->addRow(QStringLiteral("url type"), type);
        form->addRow(QStringLiteral("preferred scheme"), scheme);
        form->addRow(QStringLiteral("domain"), netloc);
        form->addRow(QStringLiteral("path"), path);
        form->addRow(QStringLiteral("parameters"), parameters);
        form->addRow(QStringLiteral("example url"), example);
        form->addRow(match_subdomains);
        form->addRow(keep_subdomains);
        form->addRow(multiple);
        form->addRow(associate);
        layout->addLayout(form);

        auto *test_row = new QWidget(this);
        auto *test_layout = new QHBoxLayout(test_row);
        test_layout->setContentsMargins(0, 0, 0, 0);
        test = new QLineEdit(test_row);
        test->setPlaceholderText(QStringLiteral("paste a url to test matching"));
        auto *test_button = new QPushButton(QStringLiteral("test"), test_row);
        test_layout->addWidget(test, 1);
        test_layout->addWidget(test_button);
        layout->addWidget(test_row);

        result = new QLabel(this);
        result->setWordWrap(true);
        layout->addWidget(result);

        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        connect(test_button, &QPushButton::clicked, this, [this]() { run_test(); });
        layout->addWidget(buttons);

        name->setText(data.value(QStringLiteral("name")).toString());
        scheme->setText(data.value(QStringLiteral("preferred_scheme")).toString());
        if (scheme->text().isEmpty()) {
            scheme->setText(QStringLiteral("https"));
        }
        netloc->setText(data.value(QStringLiteral("netloc")).toString());
        path->setText(json_to_strings(data.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
        parameters->setPlainText(data.value(QStringLiteral("parameters")).toString());
        example->setText(data.value(QStringLiteral("example_url")).toString());
        match_subdomains->setChecked(data.value(QStringLiteral("match_subdomains")).toBool());
        keep_subdomains->setChecked(data.value(QStringLiteral("keep_matched_subdomains")).toBool());
        multiple->setChecked(data.value(QStringLiteral("can_produce_multiple_files")).toBool());
        associate->setChecked(data.value(QStringLiteral("should_be_associated_with_files")).toBool(true));

        const int index = type->findData(data.value(QStringLiteral("url_type")).toInt(0));
        if (index >= 0) {
            type->setCurrentIndex(index);
        }
    }

    QJsonObject payload() const {
        QJsonObject value;
        value.insert(QStringLiteral("original"), data.value(QStringLiteral("name")).toString());
        value.insert(QStringLiteral("name"), name->text().trimmed());
        value.insert(QStringLiteral("url_type"), type->currentData().toInt());
        value.insert(QStringLiteral("preferred_scheme"), scheme->text().trimmed());
        value.insert(QStringLiteral("netloc"), netloc->text().trimmed());
        value.insert(QStringLiteral("path"), path->text().trimmed());
        value.insert(QStringLiteral("parameters"), parameters->toPlainText());
        value.insert(QStringLiteral("example_url"), example->text().trimmed());
        value.insert(QStringLiteral("match_subdomains"), match_subdomains->isChecked());
        value.insert(QStringLiteral("keep_matched_subdomains"), keep_subdomains->isChecked());
        value.insert(QStringLiteral("can_produce_multiple_files"), multiple->isChecked());
        value.insert(QStringLiteral("should_be_associated_with_files"), associate->isChecked());

        return value;
    }

private:
    void run_test() {
        const QUrl url(test->text().trimmed());

        if (!url.isValid() || url.host().isEmpty()) {
            result->setText(QStringLiteral("not a valid http(s) url"));
            return;
        }

        QString domain = netloc->text().trimmed().toLower();
        domain.remove(QStringLiteral("www."));
        QString host = url.host().toLower();
        host.remove(QStringLiteral("www."));

        if (host != domain
            && !(match_subdomains->isChecked() && host.endsWith(QStringLiteral(".") + domain))) {
            result->setText(QStringLiteral("domain does not match %1").arg(domain));
            return;
        }

        const QStringList expected = path->text().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);
        const QStringList actual = url.path().trimmed().split(QLatin1Char('/'), Qt::SkipEmptyParts);

        if (expected.size() != actual.size()) {
            result->setText(QStringLiteral("path has %1 segments; expected %2").arg(actual.size()).arg(expected.size()));
            return;
        }

        for (int index = 0; index < expected.size(); ++index) {
            const QString component = expected.at(index);

            if (component.startsWith(QLatin1Char('{')) && component.endsWith(QLatin1Char('}'))) {
                continue;
            }

            if (component != actual.at(index)) {
                result->setText(QStringLiteral("segment %1 does not match").arg(index + 1));
                return;
            }
        }

        result->setText(QStringLiteral("matches this url class"));
    }

    QJsonObject data;
    QLineEdit *name = nullptr;
    QComboBox *type = nullptr;
    QLineEdit *scheme = nullptr;
    QLineEdit *netloc = nullptr;
    QLineEdit *path = nullptr;
    QPlainTextEdit *parameters = nullptr;
    QLineEdit *example = nullptr;
    QCheckBox *match_subdomains = nullptr;
    QCheckBox *keep_subdomains = nullptr;
    QCheckBox *multiple = nullptr;
    QCheckBox *associate = nullptr;
    QLineEdit *test = nullptr;
    QLabel *result = nullptr;
};

} // namespace

namespace {

QJsonObject service_type_entry(const QJsonObject &data, int value) {
    for (const QJsonValue &entry : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject object = entry.toObject();

        if (object.value(QStringLiteral("value")).toInt() == value) {
            return object;
        }
    }

    return {};
}

QString service_family(const QJsonObject &data, int value) {
    return service_type_entry(data, value).value(QStringLiteral("family")).toString();
}

void build_service_editor(HydrusMainWindow *main, QWidget *parent, const QJsonObject &data, const QJsonObject &service) {
    const QString original = service.value(QStringLiteral("original")).toString();
    const QJsonObject dictionary = service.value(QStringLiteral("dictionary")).toObject();

    QDialog dialog(parent);
    dialog.setWindowTitle(original.isEmpty() ? QStringLiteral("add service")
                                             : QStringLiteral("edit service"));
    dialog.resize(620, 520);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(service.value(QStringLiteral("name")).toString(), &dialog);
    auto *type = new QComboBox(&dialog);

    for (const QJsonValue &entry : data.value(QStringLiteral("types")).toArray()) {
        const QJsonObject object = entry.toObject();
        type->addItem(object.value(QStringLiteral("label")).toString(),
                      object.value(QStringLiteral("value")).toInt());
    }

    const int type_index = type->findData(service.value(QStringLiteral("type_int")).toInt());
    type->setCurrentIndex(type_index >= 0 ? type_index : 0);

    auto *host = new QLineEdit(dictionary.value(QStringLiteral("host")).toString(), &dialog);
    host->setPlaceholderText(QStringLiteral("127.0.0.1"));
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(dictionary.value(QStringLiteral("port")).toInt(45871));
    auto *access_key = new QLineEdit(dictionary.value(QStringLiteral("access_key")).toString(), &dialog);
    access_key->setPlaceholderText(QStringLiteral("hex access key from the server administrator"));

    auto *update_period = new QSpinBox(&dialog);
    update_period->setRange(0, 365 * 86400);
    update_period->setValue(dictionary.value(QStringLiteral("update_period")).toInt(86400));
    update_period->setSuffix(QStringLiteral(" s"));
    auto *nullification_period = new QSpinBox(&dialog);
    nullification_period->setRange(0, 365 * 86400);
    nullification_period->setValue(dictionary.value(QStringLiteral("nullification_period")).toInt(90 * 86400));
    nullification_period->setSuffix(QStringLiteral(" s"));

    auto *max_storage = new QSpinBox(&dialog);
    max_storage->setRange(0, 1000000000);
    max_storage->setValue(dictionary.value(QStringLiteral("max_storage")).toInt());
    max_storage->setSuffix(QStringLiteral(" MB"));
    auto *log_uploader_ips = new QCheckBox(QStringLiteral("log uploader IPs"), &dialog);
    log_uploader_ips->setChecked(dictionary.value(QStringLiteral("log_uploader_ips")).toBool(false));

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("host"), host);
    form->addRow(QStringLiteral("port"), port);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("update period"), update_period);
    form->addRow(QStringLiteral("nullification period"), nullification_period);
    form->addRow(QStringLiteral("max storage"), max_storage);
    form->addRow(log_uploader_ips);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("Local services need only a name. Repository and administration "
                       "services also store the host, port and access key the client uses to "
                       "contact them."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto sync_fields = [&]() {
        const int value = type->currentData().toInt();
        const QString family = service_family(data, value);
        const bool remote = family == QStringLiteral("remote");
        const bool repository = service_type_entry(data, value)
                                    .value(QStringLiteral("label"))
                                    .toString()
                                    .contains(QStringLiteral("repository"));

        host->setEnabled(remote);
        port->setEnabled(remote);
        access_key->setEnabled(remote);
        update_period->setEnabled(repository);
        nullification_period->setEnabled(repository);
        max_storage->setEnabled(value == 1);
        log_uploader_ips->setEnabled(value == 1);
    };

    QObject::connect(type, &QComboBox::currentIndexChanged, &dialog, [&](int) { sync_fields(); });
    sync_fields();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"), QStringLiteral("Enter a name."));
            return;
        }

        const QString family = service_family(data, type->currentData().toInt());

        if (family == QStringLiteral("remote") && access_key->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need an access key."));
            return;
        }

        if (family == QStringLiteral("remote") && host->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Remote services need a host."));
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
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());

    QJsonObject next_dictionary;
    const QString family = service_family(data, type->currentData().toInt());
    const bool repository = type->currentText().contains(QStringLiteral("repository"));

    if (family == QStringLiteral("remote")) {
        next_dictionary.insert(QStringLiteral("host"), host->text().trimmed());
        next_dictionary.insert(QStringLiteral("port"), port->value());
        next_dictionary.insert(QStringLiteral("access_key"),
                               access_key->text().trimmed().toLower());
    }

    if (repository) {
        next_dictionary.insert(QStringLiteral("update_period"), update_period->value());
        next_dictionary.insert(QStringLiteral("nullification_period"), nullification_period->value());
    }

    if (type->currentData().toInt() == 1) {
        next_dictionary.insert(QStringLiteral("max_storage"), max_storage->value());
        next_dictionary.insert(QStringLiteral("log_uploader_ips"), log_uploader_ips->isChecked());
    }

    payload.insert(QStringLiteral("dictionary"), next_dictionary);
    main->notify(QStringLiteral("service_save"), QString(), json_payload(payload));
}

QString cell_value(QTableWidget *table, int row, int column) {
    QTableWidgetItem *item = table->item(row, column);

    return item == nullptr ? QString() : item->text();
}

QString cell_key(QTableWidget *table, int row) {
    QTableWidgetItem *item = table->item(row, 0);

    return item == nullptr ? QString() : item->data(Qt::UserRole + 1).toString();
}

} // namespace

HydrusServicesDialog::HydrusServicesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage services"));
    setObjectName(QStringLiteral("hydrus_services_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 720);

    auto *layout = new QVBoxLayout(this);
    tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    // Local service manager -------------------------------------------------
    auto *local = new QWidget(tabs);
    auto *local_layout = new QVBoxLayout(local);

    table = new QTableWidget(local);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("service key"),
        QStringLiteral("port"),
        QStringLiteral("settings")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    local_layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add service..."), local);
    auto *edit = new QPushButton(QStringLiteral("edit..."), local);
    auto *remove = new QPushButton(QStringLiteral("delete"), local);
    auto *specifier = new QPushButton(QStringLiteral("service specifier..."), local);
    auto *tag_filter = new QPushButton(QStringLiteral("tag filter / purge..."), local);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addWidget(specifier);
    buttons->addWidget(tag_filter);
    buttons->addStretch(1);
    local_layout->addLayout(buttons);

    connect(add, &QPushButton::clicked, this, [this]() { add_service(); });
    connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    connect(remove, &QPushButton::clicked, this, [this]() { delete_selected(); });
    connect(specifier, &QPushButton::clicked, this, [this]() { show_specifier(); });
    connect(tag_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    tabs->addTab(local, QStringLiteral("services"));

    // Remote server ---------------------------------------------------------
    auto *server = new QWidget(tabs);
    auto *server_layout = new QVBoxLayout(server);

    auto *connection = new QHBoxLayout();
    admin_address = new QLineEdit(server);
    admin_address->setPlaceholderText(QStringLiteral("https://127.0.0.1:45870"));
    admin_key = new QLineEdit(server);
    admin_key->setPlaceholderText(QStringLiteral("administration access key"));
    auto *connect_button = new QPushButton(QStringLiteral("connect"), server);
    connection->addWidget(new QLabel(QStringLiteral("address"), server));
    connection->addWidget(admin_address, 2);
    connection->addWidget(new QLabel(QStringLiteral("key"), server));
    connection->addWidget(admin_key, 2);
    connection->addWidget(connect_button);
    server_layout->addLayout(connection);

    admin_status = new QLabel(server);
    admin_status->setWordWrap(true);
    server_layout->addWidget(admin_status);

    server_layout->addWidget(new QLabel(QStringLiteral("server services"), server));
    remote_services = new QTableWidget(server);
    remote_services->setColumnCount(5);
    remote_services->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("type"),
        QStringLiteral("port"),
        QStringLiteral("service key"),
        QStringLiteral("access key")
    });
    remote_services->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    remote_services->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    remote_services->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Stretch);
    remote_services->setSelectionBehavior(QAbstractItemView::SelectRows);
    remote_services->setSelectionMode(QAbstractItemView::SingleSelection);
    remote_services->setEditTriggers(QAbstractItemView::NoEditTriggers);
    remote_services->verticalHeader()->setVisible(false);
    server_layout->addWidget(remote_services, 1);

    auto *remote_buttons = new QHBoxLayout();
    remote_key = new QLineEdit(server);
    remote_key->setPlaceholderText(QStringLiteral("selected service access key"));
    auto *use = new QPushButton(QStringLiteral("review accounts"), server);
    auto *add_remote = new QPushButton(QStringLiteral("add service..."), server);
    auto *edit_remote = new QPushButton(QStringLiteral("edit service..."), server);
    auto *remove_remote = new QPushButton(QStringLiteral("delete service"), server);
    auto *lock = new QPushButton(QStringLiteral("lock"), server);
    auto *unlock = new QPushButton(QStringLiteral("unlock"), server);
    auto *regen = new QPushButton(QStringLiteral("regen info"), server);
    auto *info = new QPushButton(QStringLiteral("service info..."), server);
    auto *remote_filter = new QPushButton(QStringLiteral("tag filter..."), server);
    auto *account_types = new QPushButton(QStringLiteral("account types..."), server);
    auto *update_period = new QPushButton(QStringLiteral("update period..."), server);
    auto *nullification_period = new QPushButton(QStringLiteral("nullification..."), server);
    remote_buttons->addWidget(remote_key, 1);
    remote_buttons->addWidget(use);
    remote_buttons->addWidget(add_remote);
    remote_buttons->addWidget(edit_remote);
    remote_buttons->addWidget(remove_remote);
    server_layout->addLayout(remote_buttons);

    auto *period_buttons = new QHBoxLayout();
    period_buttons->addWidget(lock);
    period_buttons->addWidget(unlock);
    period_buttons->addWidget(regen);
    period_buttons->addWidget(info);
    period_buttons->addWidget(remote_filter);
    period_buttons->addWidget(account_types);
    period_buttons->addWidget(update_period);
    period_buttons->addWidget(nullification_period);
    period_buttons->addStretch(1);
    server_layout->addLayout(period_buttons);

    server_layout->addWidget(new QLabel(QStringLiteral("accounts on the selected service"), server));
    accounts = new QTableWidget(server);
    accounts->setColumnCount(6);
    accounts->setHorizontalHeaderLabels({
        QStringLiteral("account key"),
        QStringLiteral("type"),
        QStringLiteral("created"),
        QStringLiteral("expires"),
        QStringLiteral("banned"),
        QStringLiteral("message")
    });
    accounts->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    accounts->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    accounts->horizontalHeader()->setSectionResizeMode(5, QHeaderView::Stretch);
    accounts->setSelectionBehavior(QAbstractItemView::SelectRows);
    accounts->setSelectionMode(QAbstractItemView::SingleSelection);
    accounts->setEditTriggers(QAbstractItemView::NoEditTriggers);
    accounts->verticalHeader()->setVisible(false);
    server_layout->addWidget(accounts, 1);

    auto *account_buttons = new QHBoxLayout();
    auto *ban = new QPushButton(QStringLiteral("ban..."), server);
    auto *unban = new QPushButton(QStringLiteral("unban"), server);
    auto *expires = new QPushButton(QStringLiteral("expires..."), server);
    auto *message = new QPushButton(QStringLiteral("message..."), server);
    auto *delete_content = new QPushButton(QStringLiteral("delete content"), server);
    auto *account_type = new QPushButton(QStringLiteral("account type..."), server);
    account_buttons->addWidget(ban);
    account_buttons->addWidget(unban);
    account_buttons->addWidget(expires);
    account_buttons->addWidget(message);
    account_buttons->addWidget(delete_content);
    account_buttons->addWidget(account_type);
    account_buttons->addStretch(1);
    server_layout->addLayout(account_buttons);

    connect(connect_button, &QPushButton::clicked, this, [this]() { connect_admin(); });
    connect(use, &QPushButton::clicked, this, [this]() { select_remote_service(); });
    connect(add_remote, &QPushButton::clicked, this, [this]() { add_remote_service(); });
    connect(edit_remote, &QPushButton::clicked, this, [this]() { edit_remote_service(); });
    connect(remove_remote, &QPushButton::clicked, this, [this]() { remove_remote_service(); });
    connect(lock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(unlock, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(regen, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(update_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(nullification_period, &QPushButton::clicked, this, [this]() { remote_service_action(); });
    connect(info, &QPushButton::clicked, this, [this]() {
        const QJsonObject admin = data.value(QStringLiteral("admin")).toObject();
        QDialog dialog(this);
        dialog.setWindowTitle(QStringLiteral("service review"));
        dialog.resize(720, 520);
        auto *layout = new QVBoxLayout(&dialog);
        auto *text = new QTextEdit(&dialog);
        text->setReadOnly(true);

        QJsonObject review;
        review.insert(QStringLiteral("options"), admin.value(QStringLiteral("options")));
        review.insert(QStringLiteral("service_info"), admin.value(QStringLiteral("service_info")));
        text->setPlainText(QString::fromUtf8(
            QJsonDocument(review).toJson(QJsonDocument::Indented)));

        layout->addWidget(text, 1);
        auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
        layout->addWidget(buttons);
        connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
        dialog.exec();
    });
    connect(remote_filter, &QPushButton::clicked, this, [this]() {
        const QString key = selected_remote_key();

        if (!key.isEmpty()) {
            main->notify(QStringLiteral("tag_filter_open"), QString(), key);
        }
    });
    connect(account_types, &QPushButton::clicked, this, [this]() {
        if (!selected_remote_key().isEmpty()) {
            main->notify(QStringLiteral("account_types_open"), QString(), selected_remote_key());
        }
    });
    connect(ban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(unban, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(expires, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(message, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(delete_content, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(account_type, &QPushButton::clicked, this, [this]() { account_action(); });
    connect(remote_services, &QTableWidget::itemSelectionChanged, this, [this]() {
        const int row = remote_services->currentRow();

        if (row >= 0) {
            remote_key->setText(cell_value(remote_services, row, 4));
        }
    });

    tabs->addTab(server, QStringLiteral("server"));

    // Credentials -----------------------------------------------------------
    auto *credentials = new QWidget(tabs);
    auto *credentials_layout = new QVBoxLayout(credentials);
    logins = new QTableWidget(credentials);
    logins->setColumnCount(5);
    logins->setHorizontalHeaderLabels({
        QStringLiteral("domain"),
        QStringLiteral("username"),
        QStringLiteral("password"),
        QStringLiteral("access key"),
        QStringLiteral("expires")
    });
    logins->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    logins->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    logins->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    logins->setSelectionBehavior(QAbstractItemView::SelectRows);
    logins->setSelectionMode(QAbstractItemView::SingleSelection);
    logins->setEditTriggers(QAbstractItemView::NoEditTriggers);
    logins->verticalHeader()->setVisible(false);
    credentials_layout->addWidget(logins, 1);

    auto *login_buttons = new QHBoxLayout();
    auto *add_login = new QPushButton(QStringLiteral("add credentials..."), credentials);
    auto *edit_login_button = new QPushButton(QStringLiteral("edit..."), credentials);
    auto *remove_login_button = new QPushButton(QStringLiteral("remove"), credentials);
    login_buttons->addWidget(add_login);
    login_buttons->addWidget(edit_login_button);
    login_buttons->addWidget(remove_login_button);
    login_buttons->addStretch(1);
    credentials_layout->addLayout(login_buttons);
    connect(add_login, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(edit_login_button, &QPushButton::clicked, this, [this]() { this->edit_login(); });
    connect(remove_login_button, &QPushButton::clicked, this, [this]() { this->remove_login(); });
    connect(logins, &QTableWidget::cellDoubleClicked, this, [this](int, int) { this->edit_login(); });

    tabs->addTab(credentials, QStringLiteral("logins"));

    auto *close = new QPushButton(QStringLiteral("close"), this);
    auto *footer = new QHBoxLayout();
    footer->addStretch(1);
    footer->addWidget(close);
    layout->addLayout(footer);
    connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    const QString requested_tab = qEnvironmentVariable("HYDRUS_QT_SERVICES_TAB");

    if (!requested_tab.isEmpty()) {
        for (int index = 0; index < tabs->count(); ++index) {
            if (tabs->tabText(index) == requested_tab) {
                tabs->setCurrentIndex(index);
                break;
            }
        }
    }

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_SERVICES_EDIT");

    if (edit_key == QStringLiteral("new")) {
        add_service();
    } else if (!edit_key.isEmpty()) {
        for (int row = 0; row < table->rowCount(); ++row) {
            if (cell_key(table, row).compare(edit_key, Qt::CaseInsensitive) == 0) {
                table->selectRow(row);
                edit_selected();
                break;
            }
        }
    }

    if (qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_SPECIFIER")) {
        show_specifier();
    }
}

void HydrusServicesDialog::set_state(const QJsonObject &state) {
    data = state;

    const QJsonArray services = state.value(QStringLiteral("services")).toArray();
    table->setRowCount(services.size());

    for (int index = 0; index < services.size(); ++index) {
        const QJsonObject service = services.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QJsonObject dictionary = service.value(QStringLiteral("dictionary")).toObject();

        auto *name_item = table->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        name_item->setData(Qt::UserRole + 1, service.value(QStringLiteral("key")).toString());

        if (service.value(QStringLiteral("builtin")).toBool()) {
            name_item->setForeground(QColor(120, 150, 190));
        } else {
            name_item->setForeground(QBrush());
        }

        table->item(index, 1)->setText(service.value(QStringLiteral("type_short")).toString());
        table->item(index, 2)->setText(service.value(QStringLiteral("key")).toString());
        table->item(index, 3)->setText(dictionary.contains(QStringLiteral("port"))
                                            ? QString::number(dictionary.value(QStringLiteral("port")).toInt())
                                            : QStringLiteral("-"));

        QString summary = QStringLiteral("-");

        if (!dictionary.isEmpty()) {
            summary = QStringLiteral("%1:%2")
                          .arg(dictionary.value(QStringLiteral("host")).toString())
                          .arg(dictionary.value(QStringLiteral("port")).toInt());

            if (dictionary.contains(QStringLiteral("update_period"))) {
                summary += QStringLiteral(" | update every %1 s")
                               .arg(dictionary.value(QStringLiteral("update_period")).toInt());
            }

            if (dictionary.contains(QStringLiteral("nullification_period"))) {
                summary += QStringLiteral(" | nullify every %1 s")
                               .arg(dictionary.value(QStringLiteral("nullification_period")).toInt());
            }
        }

        table->item(index, 4)->setText(summary);
    }

    table->resizeRowsToContents();

    const QJsonObject admin = state.value(QStringLiteral("admin")).toObject();
    admin_address->setText(admin.value(QStringLiteral("address")).toString());
    admin_key->setText(admin.value(QStringLiteral("access_key")).toString());
    admin_status->setText(admin.value(QStringLiteral("status")).toString());

    const QJsonArray remote = admin.value(QStringLiteral("services")).toArray();
    remote_services->setRowCount(remote.size());

    for (int index = 0; index < remote.size(); ++index) {
        const QJsonObject service = remote.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (remote_services->item(index, column) == nullptr) {
                remote_services->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *name_item = remote_services->item(index, 0);
        name_item->setText(service.value(QStringLiteral("name")).toString());
        name_item->setData(Qt::UserRole, service);
        remote_services->item(index, 1)->setText(service.value(QStringLiteral("type")).toString());
        remote_services->item(index, 2)->setText(QString::number(service.value(QStringLiteral("port")).toInt()));
        remote_services->item(index, 3)->setText(service.value(QStringLiteral("key")).toString());
        remote_services->item(index, 4)->setText(service.value(QStringLiteral("access_key")).toString());

        if (service.value(QStringLiteral("selected")).toBool()) {
            remote_services->selectRow(index);
        }
    }

    remote_services->resizeRowsToContents();

    if (!opened_account_types
        && qEnvironmentVariableIsSet("HYDRUS_QT_SERVICES_ACCOUNT_TYPES")
        && remote_services->rowCount() > 0) {
        opened_account_types = true;
        const QJsonObject first = remote_services->item(0, 0)->data(Qt::UserRole).toJsonObject();
        const QString key = first.value(QStringLiteral("key")).toString();
        remote_services->selectRow(0);

        QJsonObject select;
        select.insert(QStringLiteral("action"), QStringLiteral("select"));
        select.insert(QStringLiteral("service_key"), key);
        select.insert(QStringLiteral("access_key"),
                      first.value(QStringLiteral("access_key")).toString().isEmpty()
                          ? admin_key->text().trimmed()
                          : first.value(QStringLiteral("access_key")).toString());
        main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(select));
        main->notify(QStringLiteral("account_types_open"), QString(), key);
    }

    const QJsonArray account_list = admin.value(QStringLiteral("accounts")).toArray();
    accounts->setRowCount(account_list.size());

    for (int index = 0; index < account_list.size(); ++index) {
        const QJsonObject account = account_list.at(index).toObject();

        for (int column = 0; column < 6; ++column) {
            if (accounts->item(index, column) == nullptr) {
                accounts->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *key_item = accounts->item(index, 0);
        key_item->setText(account.value(QStringLiteral("key")).toString());
        key_item->setData(Qt::UserRole, account);

        const QJsonValue expires = account.value(QStringLiteral("expires"));
        accounts->item(index, 1)->setText(account.value(QStringLiteral("type")).toString());
        accounts->item(index, 2)->setText(QString::number(account.value(QStringLiteral("created")).toInt()));
        accounts->item(index, 3)->setText(expires.isNull() ? QStringLiteral("never")
                                                           : QString::number(expires.toInt()));
        accounts->item(index, 4)->setText(account.value(QStringLiteral("banned")).toBool()
                                              ? account.value(QStringLiteral("banned_reason")).toString()
                                              : QStringLiteral("-"));
        accounts->item(index, 5)->setText(account.value(QStringLiteral("message")).toString());
    }

    accounts->resizeRowsToContents();

    const QJsonArray login_list = state.value(QStringLiteral("logins")).toArray();
    logins->setRowCount(login_list.size());

    for (int index = 0; index < login_list.size(); ++index) {
        const QJsonObject login = login_list.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (logins->item(index, column) == nullptr) {
                logins->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto *domain_item = logins->item(index, 0);
        domain_item->setText(login.value(QStringLiteral("domain")).toString());
        domain_item->setData(Qt::UserRole, login);
        logins->item(index, 1)->setText(login.value(QStringLiteral("username")).toString());
        logins->item(index, 2)->setText(login.value(QStringLiteral("password")).toString().isEmpty()
                                            ? QStringLiteral("-")
                                            : QStringLiteral("********"));
        logins->item(index, 3)->setText(login.value(QStringLiteral("access_key")).toString());
        const QJsonValue expires = login.value(QStringLiteral("expires"));
        logins->item(index, 4)->setText(expires.isNull() ? QStringLiteral("never")
                                                         : QString::number(expires.toInt()));
    }

    logins->resizeRowsToContents();
}

QString HydrusServicesDialog::selected_key() const {
    const int row = table->currentRow();

    if (row < 0) {
        return {};
    }

    return cell_key(table, row);
}

QJsonObject HydrusServicesDialog::selected_remote_service() const {
    const int row = remote_services->currentRow();

    if (row < 0 || remote_services->item(row, 0) == nullptr) {
        return {};
    }

    return remote_services->item(row, 0)->data(Qt::UserRole).toJsonObject();
}

QString HydrusServicesDialog::selected_remote_key() const {
    const QJsonObject service = selected_remote_service();

    return service.value(QStringLiteral("key")).toString();
}

void HydrusServicesDialog::add_service() {
    QJsonObject service;
    service.insert(QStringLiteral("name"), QString());
    service.insert(QStringLiteral("type_int"), 2);
    service.insert(QStringLiteral("dictionary"), QJsonObject());

    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject service = table->item(row, 0)->data(Qt::UserRole).toJsonObject();
    service.insert(QStringLiteral("original"), selected_key());
    build_service_editor(main, this, data, service);
}

void HydrusServicesDialog::delete_selected() {
    const QString key = selected_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(
            this,
            QStringLiteral("delete service"),
            QStringLiteral("Delete this service? Only empty custom services can be removed."))
        != QMessageBox::Yes) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("key"), key);

    main->notify(QStringLiteral("service_delete"), QString(), json_payload(payload));
}

void HydrusServicesDialog::show_specifier() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("service specifier"));
    dialog.resize(460, 360);

    auto *layout = new QVBoxLayout(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("file domains"), QStringLiteral("local file domain"));
    type->addItem(QStringLiteral("tag domains"), QStringLiteral("local tag domain"));
    type->addItem(QStringLiteral("notes"), QStringLiteral("notes"));
    type->addItem(QStringLiteral("ratings"), QStringLiteral("ratings"));
    type->addItem(QStringLiteral("tag repositories"), QStringLiteral("tag repository"));
    type->addItem(QStringLiteral("file repositories"), QStringLiteral("file repository"));
    layout->addWidget(type);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    auto *status = new QLabel(&dialog);
    status->setWordWrap(true);
    layout->addWidget(status);

    auto refresh = [this, list, type, status]() {
        list->clear();
        const QString wanted = type->currentData().toString();

        for (const QJsonValue &entry : data.value(QStringLiteral("services")).toArray()) {
            const QJsonObject service = entry.toObject();
            const QString short_type = service.value(QStringLiteral("type_short")).toString();

            const bool matches = wanted == QStringLiteral("ratings")
                ? short_type.contains(QStringLiteral("rating"))
                : short_type == wanted;

            if (matches) {
                auto *item = new QListWidgetItem(service.value(QStringLiteral("name")).toString(), list);
                item->setData(Qt::UserRole, service.value(QStringLiteral("key")).toString());
            }
        }

        status->setText(list->count() == 0 ? QStringLiteral("no services of that type")
                                           : QStringLiteral("%1 service(s) available").arg(list->count()));
    };

    connect(type, &QComboBox::currentIndexChanged, &dialog, [refresh](int) { refresh(); });
    refresh();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    auto *copy = new QPushButton(QStringLiteral("copy key"), &dialog);
    buttons->addButton(copy, QDialogButtonBox::ActionRole);
    layout->addWidget(buttons);
    connect(copy, &QPushButton::clicked, &dialog, [list, status]() {
        QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            status->setText(QStringLiteral("select a service first"));
            return;
        }

        QApplication::clipboard()->setText(item->data(Qt::UserRole).toString());
        status->setText(QStringLiteral("service key copied to the clipboard"));
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    dialog.exec();
}

void HydrusServicesDialog::edit_login() {
    QJsonObject existing;
    const int row = logins->currentRow();

    if (row >= 0 && logins->item(row, 0) != nullptr) {
        existing = logins->item(row, 0)->data(Qt::UserRole).toJsonObject();
    }

    QDialog dialog(this);
    dialog.setWindowTitle(existing.isEmpty() ? QStringLiteral("add credentials")
                                             : QStringLiteral("edit credentials"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *domain = new QLineEdit(existing.value(QStringLiteral("domain")).toString(), &dialog);
    auto *username = new QLineEdit(existing.value(QStringLiteral("username")).toString(), &dialog);
    auto *password = new QLineEdit(existing.value(QStringLiteral("password")).toString(), &dialog);
    password->setEchoMode(QLineEdit::Password);
    auto *access_key = new QLineEdit(existing.value(QStringLiteral("access_key")).toString(), &dialog);
    auto *expires = new QSpinBox(&dialog);
    expires->setRange(0, 3650);
    expires->setSpecialValueText(QStringLiteral("never"));
    expires->setSuffix(QStringLiteral(" days"));

    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("username"), username);
    form->addRow(QStringLiteral("password"), password);
    form->addRow(QStringLiteral("access key"), access_key);
    form->addRow(QStringLiteral("expires"), expires);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (domain->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("credentials"),
                                 QStringLiteral("Enter a domain."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("username"), username->text());
    payload.insert(QStringLiteral("password"), password->text());
    payload.insert(QStringLiteral("access_key"), access_key->text().trimmed());

    if (expires->value() > 0) {
        payload.insert(QStringLiteral("expires"),
                       QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(expires->value()) * 86400);
    }

    main->notify(QStringLiteral("login_save"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_login() {
    const int row = logins->currentRow();

    if (row < 0) {
        return;
    }

    const QString domain = cell_value(logins, row, 0);

    if (domain.isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("login_remove"), QString(), domain);
}

void HydrusServicesDialog::connect_admin() {
    QJsonObject payload;
    payload.insert(QStringLiteral("address"), admin_address->text().trimmed());
    payload.insert(QStringLiteral("access_key"), admin_key->text().trimmed());

    main->notify(QStringLiteral("admin_connect"), QString(), json_payload(payload));
}

void HydrusServicesDialog::select_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    main->notify(QStringLiteral("admin_service_action"), QString(),
                 json_payload([&]() {
                     QJsonObject with_action = payload;
                     with_action.insert(QStringLiteral("action"), QStringLiteral("select"));
                     return with_action;
                 }()));
}

void HydrusServicesDialog::add_remote_service() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();

    auto *name = new QLineEdit(&dialog);
    auto *type = new QComboBox(&dialog);
    type->addItem(QStringLiteral("tag repository"), 0);
    type->addItem(QStringLiteral("file repository"), 1);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(45871);

    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("type"), type);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *note = new QLabel(
        QStringLiteral("The server returns a new access key for every service it creates; "
                       "it is stored here and used for account and option management."),
        &dialog);
    note->setWordWrap(true);
    layout->addWidget(note);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("type_int"), type->currentData().toInt());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_add_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::edit_remote_service() {
    const QJsonObject service = selected_remote_service();

    if (service.isEmpty()) {
        return;
    }

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("edit server service"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(service.value(QStringLiteral("name")).toString(), &dialog);
    auto *port = new QSpinBox(&dialog);
    port->setRange(1, 65535);
    port->setValue(service.value(QStringLiteral("port")).toInt());
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("port"), port);
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&]() {
        if (name->text().trimmed().isEmpty()) {
            QMessageBox::warning(&dialog, QStringLiteral("service"),
                                 QStringLiteral("Enter a service name."));
            return;
        }

        dialog.accept();
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("service_key"), service.value(QStringLiteral("key")).toString());
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("port"), port->value());

    main->notify(QStringLiteral("admin_edit_service"), QString(), json_payload(payload));
}

void HydrusServicesDialog::remove_remote_service() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    if (QMessageBox::question(this, QStringLiteral("delete service"),
                              QStringLiteral("Delete this service from the server? All of its "
                                             "content will be removed."))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("admin_remove_service"), QString(), key);
}

void HydrusServicesDialog::remote_service_action() {
    const QString key = selected_remote_key();

    if (key.isEmpty()) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    QString action;
    const QString text = button->text();

    if (text.startsWith(QStringLiteral("lock"))) {
        action = QStringLiteral("lock_on");
    } else if (text.startsWith(QStringLiteral("unlock"))) {
        action = QStringLiteral("lock_off");
    } else if (text.startsWith(QStringLiteral("regen"))) {
        action = QStringLiteral("regen_info");
    } else if (text.startsWith(QStringLiteral("update"))) {
        action = QStringLiteral("update_period");
    } else if (text.startsWith(QStringLiteral("nullification"))) {
        action = QStringLiteral("nullification_period");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("service_key"), key);
    payload.insert(QStringLiteral("access_key"), remote_key->text().trimmed());

    if (action.endsWith(QStringLiteral("period"))) {
        bool ok = false;
        const int seconds = QInputDialog::getInt(
            this, QStringLiteral("service period"), QStringLiteral("seconds"),
            action.startsWith(QStringLiteral("update")) ? 86400 : 90 * 86400, 0,
            365 * 86400, 60, &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("period"), seconds);
    }

    main->notify(QStringLiteral("admin_service_action"), QString(), json_payload(payload));
}

void HydrusServicesDialog::account_action() {
    const int row = accounts->currentRow();

    if (row < 0 || accounts->item(row, 0) == nullptr) {
        return;
    }

    auto *button = qobject_cast<QPushButton *>(sender());

    if (button == nullptr) {
        return;
    }

    const QJsonObject account = accounts->item(row, 0)->data(Qt::UserRole).toJsonObject();
    const QString account_key = account.value(QStringLiteral("key")).toString();
    const QString text = button->text();
    QString action;

    if (text.startsWith(QStringLiteral("ban"))) {
        action = QStringLiteral("ban");
    } else if (text.startsWith(QStringLiteral("unban"))) {
        action = QStringLiteral("unban");
    } else if (text.startsWith(QStringLiteral("expires"))) {
        action = QStringLiteral("expires");
    } else if (text.startsWith(QStringLiteral("message"))) {
        action = QStringLiteral("message");
    } else if (text.startsWith(QStringLiteral("delete"))) {
        if (QMessageBox::question(this, QStringLiteral("delete content"),
                                  QStringLiteral("Delete all content this account uploaded?"))
            != QMessageBox::Yes) {
            return;
        }

        action = QStringLiteral("delete_all_content");
    } else if (text.startsWith(QStringLiteral("account type"))) {
        action = QStringLiteral("account_type");
    }

    if (action.isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("subject_account_key"), account_key);

    if (action == QStringLiteral("ban")) {
        bool ok = false;
        const QString reason = QInputDialog::getText(
            this, QStringLiteral("ban account"), QStringLiteral("reason"),
            QLineEdit::Normal, QStringLiteral("banned by admin"), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("reason"), reason);
    } else if (action == QStringLiteral("expires")) {
        bool ok = false;
        const int days = QInputDialog::getInt(
            this, QStringLiteral("account expiry"),
            QStringLiteral("days from now (0 = never)"), 30, 0, 36500, 1, &ok);

        if (!ok) {
            return;
        }

        if (days > 0) {
            payload.insert(QStringLiteral("expires"),
                           QDateTime::currentSecsSinceEpoch() + static_cast<qint64>(days) * 86400);
        } else {
            payload.insert(QStringLiteral("expires"), QJsonValue::Null);
        }
    } else if (action == QStringLiteral("message")) {
        bool ok = false;
        const QString message = QInputDialog::getText(
            this, QStringLiteral("account message"), QStringLiteral("message"),
            QLineEdit::Normal, account.value(QStringLiteral("message")).toString(), &ok);

        if (!ok) {
            return;
        }

        payload.insert(QStringLiteral("message"), message);
    } else if (action == QStringLiteral("account_type")) {
        const QJsonArray types = data.value(QStringLiteral("admin"))
                                     .toObject()
                                     .value(QStringLiteral("account_types"))
                                     .toArray();
        QStringList titles;

        for (const QJsonValue &entry : types) {
            titles.append(entry.toObject().value(QStringLiteral("title")).toString());
        }

        if (titles.isEmpty()) {
            QMessageBox::information(this, QStringLiteral("account type"),
                                     QStringLiteral("Review the service first."));
            return;
        }

        bool ok = false;
        const QString title = QInputDialog::getItem(
            this, QStringLiteral("account type"), QStringLiteral("type"), titles,
            titles.indexOf(account.value(QStringLiteral("type")).toString()), false, &ok);

        if (!ok) {
            return;
        }

        for (const QJsonValue &entry : types) {
            const QJsonObject object = entry.toObject();

            if (object.value(QStringLiteral("title")).toString() == title) {
                payload.insert(QStringLiteral("account_type_key"),
                               object.value(QStringLiteral("key")).toString());
                break;
            }
        }
    }

    main->notify(QStringLiteral("admin_account_action"), QString(), json_payload(payload));
}

HydrusNetworkDialog::HydrusNetworkDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("manage network"));
    setObjectName(QStringLiteral("hydrus_network_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(780, 620);

    auto *layout = new QVBoxLayout(this);
    auto *tabs = new QTabWidget(this);
    layout->addWidget(tabs, 1);

    auto *connection = new QWidget(tabs);
    auto *connection_layout = new QVBoxLayout(connection);
    auto *connection_form = new QFormLayout();

    http_proxy = new QLineEdit(connection);
    https_proxy = new QLineEdit(connection);
    no_proxy = new QLineEdit(connection);
    timeout = new QSpinBox(connection);
    timeout->setRange(3, 600);
    max_jobs = new QSpinBox(connection);
    max_jobs->setRange(1, 500);
    max_jobs_domain = new QSpinBox(connection);
    max_jobs_domain->setRange(1, 100);

    connection_form->addRow(QStringLiteral("http proxy"), http_proxy);
    connection_form->addRow(QStringLiteral("https proxy"), https_proxy);
    connection_form->addRow(QStringLiteral("no proxy"), no_proxy);
    connection_form->addRow(QStringLiteral("timeout (seconds)"), timeout);
    connection_form->addRow(QStringLiteral("max network jobs"), max_jobs);
    connection_form->addRow(QStringLiteral("max jobs per domain"), max_jobs_domain);
    connection_layout->addLayout(connection_form);

    paused = new QCheckBox(QStringLiteral("pause all new network traffic"), connection);
    connection_layout->addWidget(paused);
    connection_layout->addStretch(1);
    tabs->addTab(connection, QStringLiteral("connection"));

    auto *api = new QWidget(tabs);
    auto *api_layout = new QVBoxLayout(api);
    auto *api_form = new QFormLayout();
    api_enabled = new QCheckBox(QStringLiteral("run the local Client API"), api);
    api_port = new QSpinBox(api);
    api_port->setRange(1, 65535);
    api_key = new QLineEdit(api);
    api_key->setPlaceholderText(QStringLiteral("64 hex characters; generated when empty"));
    api_status = new QLabel(api);
    api_status->setTextInteractionFlags(Qt::TextSelectableByMouse);

    api_form->addRow(api_enabled);
    api_form->addRow(QStringLiteral("port"), api_port);
    api_form->addRow(QStringLiteral("access key"), api_key);
    api_form->addRow(QStringLiteral("status"), api_status);
    api_layout->addLayout(api_form);
    api_layout->addStretch(1);
    tabs->addTab(api, QStringLiteral("client api"));

    auto *headers_page = new QWidget(tabs);
    auto *headers_layout = new QVBoxLayout(headers_page);
    headers = new QTableWidget(headers_page);
    headers->setColumnCount(2);
    headers->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("value")});
    headers->horizontalHeader()->setStretchLastSection(true);
    headers->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    headers->setSelectionBehavior(QAbstractItemView::SelectRows);
    headers->setEditTriggers(QAbstractItemView::NoEditTriggers);
    headers->verticalHeader()->setVisible(false);
    headers_layout->addWidget(headers, 1);

    auto *header_buttons = new QHBoxLayout();
    auto *add_header_button = new QPushButton(QStringLiteral("add header..."), headers_page);
    auto *remove_header_button = new QPushButton(QStringLiteral("remove"), headers_page);
    header_buttons->addWidget(add_header_button);
    header_buttons->addWidget(remove_header_button);
    header_buttons->addStretch(1);
    headers_layout->addLayout(header_buttons);
    tabs->addTab(headers_page, QStringLiteral("custom headers"));

    auto *classes_page = new QWidget(tabs);
    auto *classes_layout = new QVBoxLayout(classes_page);
    url_classes = new QTableWidget(classes_page);
    url_classes->setColumnCount(4);
    url_classes->setHorizontalHeaderLabels({QStringLiteral("name"), QStringLiteral("type"), QStringLiteral("domain"), QStringLiteral("path")});
    url_classes->horizontalHeader()->setStretchLastSection(true);
    url_classes->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    url_classes->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    url_classes->setSelectionBehavior(QAbstractItemView::SelectRows);
    url_classes->setEditTriggers(QAbstractItemView::NoEditTriggers);
    url_classes->verticalHeader()->setVisible(false);
    classes_layout->addWidget(url_classes, 1);

    auto *class_buttons = new QHBoxLayout();
    auto *add_class = new QPushButton(QStringLiteral("add url class..."), classes_page);
    auto *edit_class = new QPushButton(QStringLiteral("edit..."), classes_page);
    auto *remove_class = new QPushButton(QStringLiteral("remove"), classes_page);
    class_buttons->addWidget(add_class);
    class_buttons->addWidget(edit_class);
    class_buttons->addWidget(remove_class);
    class_buttons->addStretch(1);
    classes_layout->addLayout(class_buttons);
    tabs->addTab(classes_page, QStringLiteral("url classes"));

    auto *pools_page = new QWidget(tabs);
    auto *pools_layout = new QVBoxLayout(pools_page);
    pools = new QTableWidget(pools_page);
    pools->setColumnCount(2);
    pools->setHorizontalHeaderLabels({QStringLiteral("domain"), QStringLiteral("simultaneous downloads")});
    pools->horizontalHeader()->setStretchLastSection(true);
    pools->setSelectionBehavior(QAbstractItemView::SelectRows);
    pools->setEditTriggers(QAbstractItemView::NoEditTriggers);
    pools->verticalHeader()->setVisible(false);
    pools_layout->addWidget(pools, 1);

    auto *pool_buttons = new QHBoxLayout();
    auto *add_pool_button = new QPushButton(QStringLiteral("add domain limit..."), pools_page);
    auto *remove_pool_button = new QPushButton(QStringLiteral("remove"), pools_page);
    pool_buttons->addWidget(add_pool_button);
    pool_buttons->addWidget(remove_pool_button);
    pool_buttons->addStretch(1);
    pools_layout->addLayout(pool_buttons);
    tabs->addTab(pools_page, QStringLiteral("download pools"));

    auto *bandwidth_page = new QWidget(tabs);
    auto *bandwidth_layout = new QVBoxLayout(bandwidth_page);
    bandwidth_rules = new QTableWidget(bandwidth_page);
    bandwidth_rules->setColumnCount(5);
    bandwidth_rules->setHorizontalHeaderLabels({
        QStringLiteral("context"),
        QStringLiteral("domain"),
        QStringLiteral("type"),
        QStringLiteral("period"),
        QStringLiteral("maximum")
    });
    bandwidth_rules->horizontalHeader()->setStretchLastSection(true);
    bandwidth_rules->setSelectionBehavior(QAbstractItemView::SelectRows);
    bandwidth_rules->setEditTriggers(QAbstractItemView::NoEditTriggers);
    bandwidth_rules->verticalHeader()->setVisible(false);
    bandwidth_layout->addWidget(bandwidth_rules, 1);
    bandwidth_layout->addWidget(new QLabel(
        QStringLiteral("Data limits are in bytes. A zero period means the calendar month."),
        bandwidth_page));

    auto *bandwidth_buttons = new QHBoxLayout();
    auto *add_rule_button = new QPushButton(QStringLiteral("add rule..."), bandwidth_page);
    auto *remove_rule_button = new QPushButton(QStringLiteral("remove"), bandwidth_page);
    bandwidth_buttons->addWidget(add_rule_button);
    bandwidth_buttons->addWidget(remove_rule_button);
    bandwidth_buttons->addStretch(1);
    bandwidth_layout->addLayout(bandwidth_buttons);
    tabs->addTab(bandwidth_page, QStringLiteral("bandwidth"));

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Close, this);
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() { save(); });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::close);
    layout->addWidget(buttons);

    connect(paused, &QCheckBox::clicked, this, [this](bool) {
        main->notify(QStringLiteral("network_pause"));
    });
    connect(add_header_button, &QPushButton::clicked, this, [this]() { add_header(); });
    connect(remove_header_button, &QPushButton::clicked, this, [this]() { remove_header(); });
    connect(add_class, &QPushButton::clicked, this, [this]() { add_url_class(); });
    connect(edit_class, &QPushButton::clicked, this, [this]() { edit_url_class(); });
    connect(remove_class, &QPushButton::clicked, this, [this]() { remove_url_class(); });
    connect(add_pool_button, &QPushButton::clicked, this, [this]() { add_pool(); });
    connect(remove_pool_button, &QPushButton::clicked, this, [this]() { remove_pool(); });
    connect(add_rule_button, &QPushButton::clicked, this, [this]() { add_bandwidth_rule(); });
    connect(remove_rule_button, &QPushButton::clicked, this, [this]() { remove_bandwidth_rule(); });

    set_state(state);
}

void HydrusNetworkDialog::set_state(const QJsonObject &state) {
    data = state;
    const QJsonObject options = state.value(QStringLiteral("options")).toObject();

    http_proxy->setText(options.value(QStringLiteral("http_proxy")).toString());
    https_proxy->setText(options.value(QStringLiteral("https_proxy")).toString());
    no_proxy->setText(options.value(QStringLiteral("no_proxy")).toString());
    timeout->setValue(options.value(QStringLiteral("network_timeout")).toInt(10));
    max_jobs->setValue(options.value(QStringLiteral("max_network_jobs")).toInt(15));
    max_jobs_domain->setValue(options.value(QStringLiteral("max_network_jobs_per_domain")).toInt(3));

    api_enabled->setChecked(options.value(QStringLiteral("api_enabled")).toBool());
    api_port->setValue(options.value(QStringLiteral("api_port")).toInt(45869));
    api_key->setText(options.value(QStringLiteral("api_access_key")).toString());
    api_status->setText(state.value(QStringLiteral("api_status")).toString());

    QSignalBlocker blocker(paused);
    paused->setChecked(state.value(QStringLiteral("paused")).toBool());

    const QJsonArray header_values = state.value(QStringLiteral("headers")).toArray();
    headers->setRowCount(header_values.size());

    for (int index = 0; index < header_values.size(); ++index) {
        const QJsonObject header = header_values.at(index).toObject();

        if (headers->item(index, 0) == nullptr) {
            headers->setItem(index, 0, new QTableWidgetItem());
            headers->setItem(index, 1, new QTableWidgetItem());
        }

        headers->item(index, 0)->setText(header.value(QStringLiteral("name")).toString());
        headers->item(index, 1)->setText(header.value(QStringLiteral("value")).toString());
    }

    const QJsonArray classes = state.value(QStringLiteral("url_classes")).toArray();
    url_classes->setRowCount(classes.size());

    for (int index = 0; index < classes.size(); ++index) {
        const QJsonObject entry = classes.at(index).toObject();

        for (int column = 0; column < 4; ++column) {
            if (url_classes->item(index, column) == nullptr) {
                url_classes->setItem(index, column, new QTableWidgetItem());
            }
        }

        url_classes->item(index, 0)->setText(entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("name")).toString());
        url_classes->item(index, 1)->setText(entry.value(QStringLiteral("url_type_name")).toString());
        url_classes->item(index, 2)->setText(entry.value(QStringLiteral("netloc")).toString());
        url_classes->item(index, 3)->setText(json_to_strings(entry.value(QStringLiteral("components")).toArray()).join(QStringLiteral("/")));
    }

    const QJsonArray pool_values = state.value(QStringLiteral("download_pools")).toArray();
    pools->setRowCount(pool_values.size());

    for (int index = 0; index < pool_values.size(); ++index) {
        const QJsonObject entry = pool_values.at(index).toObject();

        for (int column = 0; column < 2; ++column) {
            if (pools->item(index, column) == nullptr) {
                pools->setItem(index, column, new QTableWidgetItem());
            }
        }

        pools->item(index, 0)->setText(entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 0)->setData(Qt::UserRole, entry.value(QStringLiteral("domain")).toString());
        pools->item(index, 1)->setText(QString::number(entry.value(QStringLiteral("limit")).toInt()));
    }

    const QJsonArray rules = state.value(QStringLiteral("bandwidth_rules")).toArray();
    bandwidth_rules->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject entry = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (bandwidth_rules->item(index, column) == nullptr) {
                bandwidth_rules->setItem(index, column, new QTableWidgetItem());
            }
        }

        const QString context_type = entry.value(QStringLiteral("context_type")).toString();
        const QString context = entry.value(QStringLiteral("context")).toString();
        const QString bandwidth_type = entry.value(QStringLiteral("bandwidth_type")).toString();
        const QJsonValue period_value = entry.value(QStringLiteral("time_delta"));
        const qint64 max_value = entry.value(QStringLiteral("max_allowed")).toVariant().toLongLong();

        bandwidth_rules->item(index, 0)->setText(context_type);
        bandwidth_rules->item(index, 0)->setData(Qt::UserRole, context_type);
        bandwidth_rules->item(index, 1)->setText(context);
        bandwidth_rules->item(index, 1)->setData(Qt::UserRole, context);
        bandwidth_rules->item(index, 2)->setText(bandwidth_type);
        bandwidth_rules->item(index, 2)->setData(Qt::UserRole, bandwidth_type);
        bandwidth_rules->item(index, 3)->setText(
            period_value.isNull() || period_value.toInt() <= 0
                ? QStringLiteral("month")
                : QStringLiteral("%1 s").arg(period_value.toInt()));
        bandwidth_rules->item(index, 4)->setText(
            bandwidth_type == QStringLiteral("data")
                ? QStringLiteral("%1 MB").arg(max_value / (1024 * 1024))
                : QString::number(max_value));
    }

    headers->resizeRowsToContents();
    url_classes->resizeRowsToContents();
    pools->resizeRowsToContents();
    bandwidth_rules->resizeRowsToContents();
}

void HydrusNetworkDialog::save() {
    QJsonObject booleans;
    booleans.insert(QStringLiteral("api_enabled"), api_enabled->isChecked());

    QJsonObject integers;
    integers.insert(QStringLiteral("network_timeout"), timeout->value());
    integers.insert(QStringLiteral("max_network_jobs"), max_jobs->value());
    integers.insert(QStringLiteral("max_network_jobs_per_domain"), max_jobs_domain->value());
    integers.insert(QStringLiteral("api_port"), api_port->value());

    QJsonObject strings;
    strings.insert(QStringLiteral("api_access_key"), api_key->text());

    QJsonObject noneable_strings;
    noneable_strings.insert(QStringLiteral("http_proxy"), http_proxy->text());
    noneable_strings.insert(QStringLiteral("https_proxy"), https_proxy->text());
    noneable_strings.insert(QStringLiteral("no_proxy"), no_proxy->text());

    QJsonObject payload;
    payload.insert(QStringLiteral("booleans"), booleans);
    payload.insert(QStringLiteral("integers"), integers);
    payload.insert(QStringLiteral("strings"), strings);
    payload.insert(QStringLiteral("noneable_strings"), noneable_strings);

    main->notify(QStringLiteral("network_save"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_header() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("add custom header"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *name = new QLineEdit(&dialog);
    auto *value = new QLineEdit(&dialog);
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("value"), value);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || name->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), name->text().trimmed());
    payload.insert(QStringLiteral("value"), value->text());

    main->notify(QStringLiteral("network_header_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_header() {
    const int row = headers->currentRow();

    if (row < 0 || headers->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), headers->item(row, 0)->text());

    main->notify(QStringLiteral("network_header_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_url_class() {
    HydrusUrlClassDialog dialog(QJsonObject(), data.value(QStringLiteral("url_types")).toArray(), this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload = dialog.payload();

    if (payload.value(QStringLiteral("netloc")).toString().isEmpty()) {
        return;
    }

    main->notify(QStringLiteral("network_url_class_add"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::edit_url_class() {
    const int row = url_classes->currentRow();
    const QJsonArray classes = data.value(QStringLiteral("url_classes")).toArray();

    if (row < 0 || row >= classes.size()) {
        return;
    }

    HydrusUrlClassDialog dialog(
        classes.at(row).toObject(),
        data.value(QStringLiteral("url_types")).toArray(),
        this);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    main->notify(
        QStringLiteral("network_url_class_save"),
        QString(),
        json_payload(dialog.payload()));
}

void HydrusNetworkDialog::remove_url_class() {
    const int row = url_classes->currentRow();

    if (row < 0 || url_classes->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("name"), url_classes->item(row, 0)->text());

    main->notify(QStringLiteral("network_url_class_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_pool() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("domain download pool"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *domain = new QLineEdit(&dialog);
    domain->setPlaceholderText(QStringLiteral("example.com"));
    auto *limit = new QSpinBox(&dialog);
    limit->setRange(1, 64);
    limit->setValue(2);
    form->addRow(QStringLiteral("domain"), domain);
    form->addRow(QStringLiteral("simultaneous downloads"), limit);
    layout->addLayout(form);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted || domain->text().trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), domain->text().trimmed());
    payload.insert(QStringLiteral("limit"), limit->value());
    main->notify(QStringLiteral("network_pool_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_pool() {
    const int row = pools->currentRow();

    if (row < 0 || pools->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("domain"), pools->item(row, 0)->data(Qt::UserRole).toString());
    main->notify(QStringLiteral("network_pool_remove"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::add_bandwidth_rule() {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("bandwidth rule"));
    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *context_type = new QComboBox(&dialog);
    context_type->addItem(QStringLiteral("global"), QStringLiteral("global"));
    context_type->addItem(QStringLiteral("domain"), QStringLiteral("domain"));
    context_type->addItem(QStringLiteral("hydrus"), QStringLiteral("hydrus"));
    auto *context = new QLineEdit(&dialog);
    context->setPlaceholderText(QStringLiteral("domain name (domain rules only)"));
    auto *bandwidth_type = new QComboBox(&dialog);
    bandwidth_type->addItem(QStringLiteral("requests"), QStringLiteral("requests"));
    bandwidth_type->addItem(QStringLiteral("data"), QStringLiteral("data"));
    auto *period = new QSpinBox(&dialog);
    period->setRange(0, 30 * 86400);
    period->setValue(1);
    period->setSpecialValueText(QStringLiteral("calendar month"));
    auto *maximum = new QSpinBox(&dialog);
    maximum->setRange(0, 2'000'000'000);
    maximum->setValue(1);
    auto *hint = new QLabel(QStringLiteral("Data maximums are in MB; request maximums are counts."), &dialog);
    hint->setWordWrap(true);
    form->addRow(QStringLiteral("context"), context_type);
    form->addRow(QStringLiteral("domain"), context);
    form->addRow(QStringLiteral("type"), bandwidth_type);
    form->addRow(QStringLiteral("period (seconds)"), period);
    form->addRow(QStringLiteral("maximum"), maximum);
    layout->addLayout(form);
    layout->addWidget(hint);
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    connect(bandwidth_type, QOverload<int>::of(&QComboBox::currentIndexChanged), &dialog, [maximum, bandwidth_type](int) {
        if (bandwidth_type->currentData().toString() == QStringLiteral("data")) {
            maximum->setRange(0, 2'000'000);
            maximum->setValue(1024);
        } else {
            maximum->setRange(0, 2'000'000'000);
            maximum->setValue(1);
        }
    });

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), context_type->currentData().toString());
    payload.insert(QStringLiteral("context"), context->text().trimmed());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_type->currentData().toString());

    if (period->value() <= 0) {
        payload.insert(QStringLiteral("time_delta"), QJsonValue::Null);
    } else {
        payload.insert(QStringLiteral("time_delta"), period->value());
    }

    payload.insert(
        QStringLiteral("max_allowed"),
        bandwidth_type->currentData().toString() == QStringLiteral("data")
            ? static_cast<qint64>(maximum->value()) * 1024 * 1024
            : static_cast<qint64>(maximum->value()));

    main->notify(QStringLiteral("network_rule_set"), QString(), json_payload(payload));
}

void HydrusNetworkDialog::remove_bandwidth_rule() {
    const int row = bandwidth_rules->currentRow();

    if (row < 0 || bandwidth_rules->item(row, 0) == nullptr) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("context_type"), bandwidth_rules->item(row, 0)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("context"), bandwidth_rules->item(row, 1)->data(Qt::UserRole).toString());
    payload.insert(QStringLiteral("bandwidth_type"), bandwidth_rules->item(row, 2)->data(Qt::UserRole).toString());

    main->notify(QStringLiteral("network_rule_remove"), QString(), json_payload(payload));
}

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

// ---------------------------------------------------------------------------
// HydrusDuplicatesRulesDialog and the auto-resolution editors
// ---------------------------------------------------------------------------

namespace {

QComboBox *catalog_combo(const QJsonObject &catalog, const QString &key, QWidget *parent) {
    auto *combo = new QComboBox(parent);

    for (const QJsonValue &entry : catalog.value(key).toArray()) {
        const QJsonObject object = entry.toObject();
        combo->addItem(object.value(QStringLiteral("label")).toString(),
                       object.value(QStringLiteral("value")).toString());
    }

    return combo;
}

void select_combo(QComboBox *combo, const QString &value) {
    const int index = combo->findData(value);

    if (index >= 0) {
        combo->setCurrentIndex(index);
    }
}

QString comparator_pretty(const QJsonObject &comparator) {
    const QString kind = comparator.value(QStringLiteral("kind")).toString();

    if (kind == QLatin1String("group")) {
        const QString mode = comparator.value(QStringLiteral("mode")).toString();
        const int count = comparator.value(QStringLiteral("comparators")).toArray().size();

        return QStringLiteral("%1 group (%2 condition(s))")
            .arg(mode == QLatin1String("any") ? QStringLiteral("OR") : QStringLiteral("AND"))
            .arg(count);
    }

    if (kind == QLatin1String("filetype")) {
        return comparator.value(QStringLiteral("same")).toBool(true)
                   ? QStringLiteral("A and B have the same filetype")
                   : QStringLiteral("A and B have different filetypes");
    }

    const QString property = comparator.value(QStringLiteral("property")).toString();
    const QString op = comparator.value(QStringLiteral("operator")).toString(QStringLiteral(">"));

    if (kind == QLatin1String("relative")) {
        const double multiplier = comparator.value(QStringLiteral("multiplier")).toDouble(1.0);
        const double delta = comparator.value(QStringLiteral("delta")).toDouble(0.0);
        QString right = QStringLiteral("B");

        if (qAbs(multiplier - 1.0) > 0.000001) {
            right = QStringLiteral("%1x %2").arg(multiplier).arg(right);
        }

        if (delta > 0.0) {
            right += QStringLiteral(" +%1").arg(delta);
        } else if (delta < 0.0) {
            right += QStringLiteral(" -%1").arg(-delta);
        }

        return QStringLiteral("A %1 %2 %3").arg(property, op, right);
    }

    const QString target = comparator.value(QStringLiteral("looking_at")).toString(QStringLiteral("a"));
    const QString value = property == QLatin1String("mime")
                              ? comparator.value(QStringLiteral("text")).toString()
                              : QString::number(comparator.value(QStringLiteral("value")).toDouble());

    return QStringLiteral("%1 %2 %3 %4")
        .arg(target == QLatin1String("b") ? QStringLiteral("B")
             : target == QLatin1String("either") ? QStringLiteral("either")
                                                 : QStringLiteral("A"),
             property,
             op,
             value);
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog);

QJsonObject edit_condition_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison"));
    dialog.resize(460, 300);

    auto *layout = new QVBoxLayout(&dialog);

    auto *kind = new QComboBox(&dialog);
    kind->addItem(QStringLiteral("one file condition"), QStringLiteral("one_file"));
    kind->addItem(QStringLiteral("compare A and B"), QStringLiteral("relative"));
    kind->addItem(QStringLiteral("filetype check"), QStringLiteral("filetype"));
    select_combo(kind, initial.value(QStringLiteral("kind")).toString(QStringLiteral("one_file")));
    layout->addWidget(kind);

    auto *one_file = new QWidget(&dialog);
    auto *one_form = new QFormLayout(one_file);
    auto *target = catalog_combo(catalog, QStringLiteral("looking_at"), one_file);
    auto *property = catalog_combo(catalog, QStringLiteral("properties"), one_file);
    auto *operator_combo = catalog_combo(catalog, QStringLiteral("operators"), one_file);
    auto *value = new QLineEdit(one_file);
    auto *text = new QLineEdit(one_file);
    text->setPlaceholderText(QStringLiteral("image, video, png, image/jpeg ..."));
    one_form->addRow(QStringLiteral("test"), target);
    one_form->addRow(QStringLiteral("property"), property);
    one_form->addRow(QStringLiteral("operator"), operator_combo);
    one_form->addRow(QStringLiteral("value"), value);
    one_form->addRow(QStringLiteral("mime / filetype"), text);
    layout->addWidget(one_file);

    auto *relative = new QWidget(&dialog);
    auto *relative_form = new QFormLayout(relative);
    auto *relative_property = catalog_combo(catalog, QStringLiteral("properties"), relative);
    auto *relative_operator = catalog_combo(catalog, QStringLiteral("operators"), relative);
    auto *multiplier = new QDoubleSpinBox(relative);
    multiplier->setRange(0.0, 1000.0);
    multiplier->setDecimals(3);
    multiplier->setValue(1.0);
    auto *delta = new QDoubleSpinBox(relative);
    delta->setRange(-1000000000.0, 1000000000.0);
    delta->setDecimals(2);
    relative_form->addRow(QStringLiteral("property"), relative_property);
    relative_form->addRow(QStringLiteral("operator"), relative_operator);
    relative_form->addRow(QStringLiteral("B multiplier"), multiplier);
    relative_form->addRow(QStringLiteral("B delta"), delta);
    layout->addWidget(relative);

    auto *filetype = new QWidget(&dialog);
    auto *filetype_form = new QFormLayout(filetype);
    auto *same = new QCheckBox(QStringLiteral("A and B have the same filetype"), filetype);
    same->setChecked(initial.value(QStringLiteral("same")).toBool(true));
    filetype_form->addRow(same);
    layout->addWidget(filetype);

    auto update_kind = [kind, one_file, relative, filetype]() {
        const QString current = kind->currentData().toString();
        one_file->setVisible(current == QLatin1String("one_file"));
        relative->setVisible(current == QLatin1String("relative"));
        filetype->setVisible(current == QLatin1String("filetype"));
    };
    QObject::connect(kind, &QComboBox::currentIndexChanged, &dialog, [update_kind](int) { update_kind(); });

    select_combo(target, initial.value(QStringLiteral("looking_at")).toString(QStringLiteral("a")));
    select_combo(property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(relative_property, initial.value(QStringLiteral("property")).toString(QStringLiteral("size")));
    select_combo(operator_combo, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    select_combo(relative_operator, initial.value(QStringLiteral("operator")).toString(QStringLiteral(">")));
    value->setText(QString::number(initial.value(QStringLiteral("value")).toDouble()));
    text->setText(initial.value(QStringLiteral("text")).toString());
    multiplier->setValue(initial.value(QStringLiteral("multiplier")).toDouble(1.0));
    delta->setValue(initial.value(QStringLiteral("delta")).toDouble(0.0));

    update_kind();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return {};
    }

    const QString current = kind->currentData().toString();
    QJsonObject result;
    result.insert(QStringLiteral("kind"), current);

    if (current == QLatin1String("one_file")) {
        result.insert(QStringLiteral("looking_at"), target->currentData().toString());
        result.insert(QStringLiteral("property"), property->currentData().toString());
        result.insert(QStringLiteral("operator"), operator_combo->currentData().toString());
        result.insert(QStringLiteral("value"), value->text().trimmed().toDouble());
        result.insert(QStringLiteral("text"), text->text().trimmed());
    } else if (current == QLatin1String("relative")) {
        result.insert(QStringLiteral("property"), relative_property->currentData().toString());
        result.insert(QStringLiteral("operator"), relative_operator->currentData().toString());
        result.insert(QStringLiteral("multiplier"), multiplier->value());
        result.insert(QStringLiteral("delta"), delta->value());
    } else {
        result.insert(QStringLiteral("same"), same->isChecked());
    }

    return result;
}

QJsonObject edit_group_dialog(QWidget *parent, const QJsonObject &initial, const QJsonObject &catalog) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("edit comparison group"));
    dialog.resize(560, 420);

    auto *layout = new QVBoxLayout(&dialog);
    auto *form = new QFormLayout();
    auto *mode = catalog_combo(catalog, QStringLiteral("group_modes"), &dialog);
    select_combo(mode, initial.value(QStringLiteral("mode")).toString(QStringLiteral("all")));
    form->addRow(QStringLiteral("group mode"), mode);
    layout->addLayout(form);

    auto *list = new QListWidget(&dialog);
    layout->addWidget(list, 1);

    QJsonArray comparators = initial.value(QStringLiteral("comparators")).toArray();

    auto refresh = [list, &comparators]() {
        list->clear();

        for (const QJsonValue &value : comparators) {
            auto *item = new QListWidgetItem(comparator_pretty(value.toObject()), list);
            item->setData(Qt::UserRole, QString::fromUtf8(
                                           QJsonDocument(value.toObject()).toJson(QJsonDocument::Compact)));
        }
    };

    auto *buttons_row = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add condition"), &dialog);
    auto *edit = new QPushButton(QStringLiteral("edit"), &dialog);
    auto *remove = new QPushButton(QStringLiteral("remove"), &dialog);
    buttons_row->addWidget(add);
    buttons_row->addWidget(edit);
    buttons_row->addWidget(remove);
    buttons_row->addStretch(1);
    layout->addLayout(buttons_row);

    QObject::connect(add, &QPushButton::clicked, &dialog, [&]() {
        const QJsonObject condition = edit_condition_dialog(&dialog, QJsonObject(), catalog);

        if (!condition.isEmpty()) {
            comparators.append(condition);
            refresh();
        }
    });

    QObject::connect(edit, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row < 0) {
            return;
        }

        const QJsonObject current = comparators.at(row).toObject();
        const QJsonObject changed = current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
                                        ? edit_group_dialog(&dialog, current, catalog)
                                        : edit_condition_dialog(&dialog, current, catalog);

        if (!changed.isEmpty()) {
            comparators.replace(row, changed);
            refresh();
        }
    });

    QObject::connect(remove, &QPushButton::clicked, &dialog, [&]() {
        const int row = list->currentRow();

        if (row >= 0) {
            comparators.removeAt(row);
            refresh();
        }
    });

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);

    refresh();

    if (dialog.exec() != QDialog::Accepted || comparators.isEmpty()) {
        return {};
    }

    QJsonObject result;
    result.insert(QStringLiteral("kind"), QStringLiteral("group"));
    result.insert(QStringLiteral("mode"), mode->currentData().toString());
    result.insert(QStringLiteral("comparators"), comparators);

    return result;
}

} // namespace

HydrusAutoResolutionRuleDialog::HydrusAutoResolutionRuleDialog(
    HydrusMainWindow *main_window,
    HydrusDuplicatesRulesDialog *rules,
    const QJsonObject &rule,
    const QJsonObject &rule_catalog,
    QWidget *parent)
    : QDialog(parent), main(main_window), rules_dialog(rules), catalog(rule_catalog) {
    setWindowTitle(QStringLiteral("edit auto-resolution rule"));
    setObjectName(QStringLiteral("hydrus_auto_resolution_rule_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(820, 680);

    original_name = rule.value(QStringLiteral("name")).toString();
    selector = rule.value(QStringLiteral("pair_selector")).toArray();

    auto *layout = new QVBoxLayout(this);

    auto *warning = new QLabel(
        QStringLiteral("Rules test both ways around. Every comparator in the comparison list must match "
                       "(AND); add a group to mix AND/OR conditions. Preview before enabling automatic actions."),
        this);
    warning->setWordWrap(true);
    layout->addWidget(warning);

    auto *form = new QFormLayout();
    name = new QLineEdit(rule.value(QStringLiteral("name")).toString(), this);
    paused = new QCheckBox(QStringLiteral("paused"), this);
    paused->setChecked(rule.value(QStringLiteral("paused")).toBool());
    operation = catalog_combo(catalog, QStringLiteral("operation_modes"), this);
    select_combo(operation, rule.value(QStringLiteral("operation_mode")).toString(QStringLiteral("semi-automatic")));
    max_pending = new QSpinBox(this);
    max_pending->setRange(1, 10000);
    max_pending->setValue(rule.value(QStringLiteral("max_pending_pairs")).toInt(500));
    max_pending->setToolTip(QStringLiteral("semi-automatic queue cap for this rule"));
    form->addRow(QStringLiteral("name"), name);
    form->addRow(QStringLiteral("paused"), paused);
    form->addRow(QStringLiteral("operation"), operation);
    form->addRow(QStringLiteral("max pending pairs"), max_pending);
    layout->addLayout(form);

    auto *tabs = new QTabWidget(this);

    auto *search_tab = new QWidget(tabs);
    auto *search_form = new QFormLayout(search_tab);
    search_query = new QLineEdit(rule.value(QStringLiteral("search_query")).toString(), search_tab);
    search_query->setPlaceholderText(QStringLiteral("e.g. system:filetype = image, or leave empty for every pair"));
    search_mode = catalog_combo(catalog, QStringLiteral("search_modes"), search_tab);
    if (search_mode->count() == 0) {
        search_mode->addItem(QStringLiteral("both files match"), QStringLiteral("both"));
        search_mode->addItem(QStringLiteral("either file matches"), QStringLiteral("either"));
    }
    select_combo(search_mode, rule.value(QStringLiteral("search_mode")).toString(QStringLiteral("both")));
    max_distance = new QSpinBox(search_tab);
    max_distance->setRange(0, 1000);
    max_distance->setValue(rule.value(QStringLiteral("max_distance")).toInt(0));
    max_distance->setSpecialValueText(QStringLiteral("use the page/global distance"));
    search_form->addRow(QStringLiteral("search predicate"), search_query);
    search_form->addRow(QStringLiteral("files must"), search_mode);
    search_form->addRow(QStringLiteral("maximum distance"), max_distance);
    tabs->addTab(search_tab, QStringLiteral("search"));

    auto *comparison_tab = new QWidget(tabs);
    auto *comparison_layout = new QVBoxLayout(comparison_tab);
    auto *comparison_help = new QLabel(
        QStringLiteral("The pair must satisfy every row. A single row may be a one-file condition, "
                       "an A/B comparison or an AND/OR group."),
        comparison_tab);
    comparison_help->setWordWrap(true);
    comparison_layout->addWidget(comparison_help);
    comparators = new QListWidget(comparison_tab);
    comparison_layout->addWidget(comparators, 1);
    auto *comparison_buttons = new QHBoxLayout();
    auto *add_condition = new QPushButton(QStringLiteral("add condition..."), comparison_tab);
    auto *add_group = new QPushButton(QStringLiteral("add group..."), comparison_tab);
    auto *edit_condition = new QPushButton(QStringLiteral("edit"), comparison_tab);
    auto *remove_condition = new QPushButton(QStringLiteral("remove"), comparison_tab);
    comparison_buttons->addWidget(add_condition);
    comparison_buttons->addWidget(add_group);
    comparison_buttons->addWidget(edit_condition);
    comparison_buttons->addWidget(remove_condition);
    comparison_buttons->addStretch(1);
    comparison_layout->addLayout(comparison_buttons);
    tabs->addTab(comparison_tab, QStringLiteral("comparison"));

    auto *action_tab = new QWidget(tabs);
    auto *action_form = new QFormLayout(action_tab);
    action = catalog_combo(catalog, QStringLiteral("actions"), action_tab);
    select_combo(action, rule.value(QStringLiteral("action")).toString(QStringLiteral("better")));
    delete_a = new QCheckBox(QStringLiteral("also delete A after the action"), action_tab);
    delete_b = new QCheckBox(QStringLiteral("also delete B after the action"), action_tab);
    delete_a->setChecked(rule.value(QStringLiteral("delete_a")).toBool());
    delete_b->setChecked(rule.value(QStringLiteral("delete_b")).toBool());
    action_form->addRow(QStringLiteral("action"), action);
    action_form->addRow(delete_a);
    action_form->addRow(delete_b);
    tabs->addTab(action_tab, QStringLiteral("action"));

    auto *preview_tab = new QWidget(tabs);
    auto *preview_layout = new QVBoxLayout(preview_tab);
    auto *preview_help = new QLabel(
        QStringLiteral("Test this rule against the current duplicate queue. Nothing is changed."),
        preview_tab);
    preview_help->setWordWrap(true);
    preview_layout->addWidget(preview_help);
    preview_label = new QLabel(QStringLiteral("not tested yet"), preview_tab);
    preview_label->setWordWrap(true);
    preview_label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    preview_layout->addWidget(preview_label);

    preview_layout->addWidget(new QLabel(QStringLiteral("pairs that will be actioned"), preview_tab));
    preview_pass = new QTableWidget(preview_tab);
    preview_pass->setColumnCount(4);
    preview_pass->setHorizontalHeaderLabels({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance"),
        QStringLiteral("action")
    });
    preview_pass->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    preview_pass->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    preview_pass->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    preview_pass->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    preview_pass->setColumnWidth(0, 116);
    preview_pass->setColumnWidth(1, 116);
    preview_pass->setSelectionBehavior(QAbstractItemView::SelectRows);
    preview_pass->setSelectionMode(QAbstractItemView::ExtendedSelection);
    preview_pass->setEditTriggers(QAbstractItemView::NoEditTriggers);
    preview_pass->verticalHeader()->setVisible(false);
    preview_pass->setIconSize(QSize(110, 90));
    preview_layout->addWidget(preview_pass, 1);

    preview_layout->addWidget(new QLabel(QStringLiteral("pairs that will be skipped"), preview_tab));
    preview_fail = new QTableWidget(preview_tab);
    preview_fail->setColumnCount(3);
    preview_fail->setHorizontalHeaderLabels({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance")
    });
    preview_fail->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    preview_fail->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    preview_fail->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    preview_fail->setColumnWidth(0, 116);
    preview_fail->setColumnWidth(1, 116);
    preview_fail->setSelectionBehavior(QAbstractItemView::SelectRows);
    preview_fail->setSelectionMode(QAbstractItemView::ExtendedSelection);
    preview_fail->setEditTriggers(QAbstractItemView::NoEditTriggers);
    preview_fail->verticalHeader()->setVisible(false);
    preview_fail->setIconSize(QSize(110, 90));
    preview_layout->addWidget(preview_fail, 1);

    install_pair_list_actions(preview_pass, main);
    install_pair_list_actions(preview_fail, main);

    auto *run_preview = new QPushButton(QStringLiteral("test against the queue"), preview_tab);
    preview_layout->addWidget(run_preview, 0, Qt::AlignLeft);
    tabs->addTab(preview_tab, QStringLiteral("preview"));

    layout->addWidget(tabs, 1);

    auto *footer = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *save = new QPushButton(QStringLiteral("save rule"), this);
    footer->addStretch(1);
    footer->addWidget(cancel);
    footer->addWidget(save);
    layout->addLayout(footer);

    QObject::connect(add_condition, &QPushButton::clicked, this, [this]() { add_comparator(); });
    QObject::connect(add_group, &QPushButton::clicked, this, [this]() { this->add_group(); });
    QObject::connect(edit_condition, &QPushButton::clicked, this, [this]() { edit_selected_comparator(); });
    QObject::connect(remove_condition, &QPushButton::clicked, this, [this]() { remove_selected_comparator(); });
    QObject::connect(comparators, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) {
        edit_selected_comparator();
    });
    QObject::connect(run_preview, &QPushButton::clicked, this, [this]() { this->run_preview(); });
    QObject::connect(save, &QPushButton::clicked, this, [this]() { this->save(); });
    QObject::connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(operation, &QComboBox::currentIndexChanged, this, [this](int) {
        max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    });

    max_pending->setEnabled(operation->currentData().toString() != QLatin1String("fully automatic"));
    refresh_comparators();

    if (qEnvironmentVariableIsSet("HYDRUS_QT_DUPLICATES_RULE_PREVIEW")) {
        tabs->setCurrentIndex(3);
        QTimer::singleShot(250, this, [this]() { this->run_preview(); });
    }

    const QString tab = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_TAB");
    if (tab == QStringLiteral("comparison")) {
        tabs->setCurrentIndex(1);
    } else if (tab == QStringLiteral("action")) {
        tabs->setCurrentIndex(2);
    } else if (tab == QStringLiteral("preview")) {
        tabs->setCurrentIndex(3);
    }
}

void HydrusAutoResolutionRuleDialog::refresh_comparators() {
    comparators->clear();

    for (const QJsonValue &value : selector) {
        comparators->addItem(comparator_pretty(value.toObject()));
    }
}

void HydrusAutoResolutionRuleDialog::add_comparator() {
    QJsonObject initial;
    initial.insert(QStringLiteral("kind"), QStringLiteral("one_file"));
    initial.insert(QStringLiteral("looking_at"), QStringLiteral("a"));
    initial.insert(QStringLiteral("property"), QStringLiteral("size"));
    initial.insert(QStringLiteral("operator"), QStringLiteral(">"));
    initial.insert(QStringLiteral("value"), 0.0);

    const QJsonObject result = edit_condition_dialog(this, initial, catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::add_group() {
    const QJsonObject result = edit_group_dialog(this, QJsonObject(), catalog);

    if (!result.isEmpty()) {
        selector.append(result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::edit_selected_comparator() {
    const int row = comparators->currentRow();

    if (row < 0 || row >= selector.size()) {
        return;
    }

    const QJsonObject current = selector.at(row).toObject();
    const QJsonObject result =
        current.value(QStringLiteral("kind")).toString() == QLatin1String("group")
            ? edit_group_dialog(this, current, catalog)
            : edit_condition_dialog(this, current, catalog);

    if (!result.isEmpty()) {
        selector.replace(row, result);
        refresh_comparators();
    }
}

void HydrusAutoResolutionRuleDialog::remove_selected_comparator() {
    const int row = comparators->currentRow();

    if (row >= 0 && row < selector.size()) {
        selector.removeAt(row);
        refresh_comparators();
    }
}

QJsonObject HydrusAutoResolutionRuleDialog::rule_json() const {
    QJsonObject result;
    result.insert(QStringLiteral("name"), name->text().trimmed());
    result.insert(QStringLiteral("paused"), paused->isChecked());
    result.insert(QStringLiteral("operation_mode"), operation->currentData().toString());
    result.insert(QStringLiteral("max_pending_pairs"), max_pending->value());
    result.insert(QStringLiteral("search_query"), search_query->text().trimmed());
    result.insert(QStringLiteral("search_mode"), search_mode->currentData().toString());
    result.insert(QStringLiteral("max_distance"), max_distance->value());
    result.insert(QStringLiteral("pair_selector"), selector);
    result.insert(QStringLiteral("action"), action->currentData().toString());
    result.insert(QStringLiteral("delete_a"), delete_a->isChecked());
    result.insert(QStringLiteral("delete_b"), delete_b->isChecked());

    return result;
}

void HydrusAutoResolutionRuleDialog::run_preview() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    preview_label->setText(QStringLiteral("testing..."));

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_preview"), QString(), json_payload(payload));
}

void HydrusAutoResolutionRuleDialog::set_preview(const QJsonObject &preview) {
    const QString status = preview.value(QStringLiteral("status")).toString();

    if (status.isEmpty()) {
        preview_label->setText(QStringLiteral("no preview result"));
        preview_pass->setRowCount(0);
        preview_fail->setRowCount(0);
        return;
    }

    QStringList lines;
    lines.append(status);

    const QString selector_text = preview.value(QStringLiteral("selector")).toString();
    if (!selector_text.isEmpty()) {
        lines.append(QStringLiteral("selector: %1").arg(selector_text));
    }

    const QString search = preview.value(QStringLiteral("search")).toString();
    if (!search.isEmpty()) {
        lines.append(QStringLiteral("search: %1").arg(search));
    }

    preview_label->setText(lines.join(QLatin1Char('\n')));

    fill_pair_table(
        preview_pass,
        preview.value(QStringLiteral("samples")).toArray(),
        2,
        {QStringLiteral("distance"), QStringLiteral("decision")});
    fill_pair_table(
        preview_fail,
        preview.value(QStringLiteral("failed_samples")).toArray(),
        2,
        {QStringLiteral("distance")});
}

void HydrusAutoResolutionRuleDialog::save() {
    if (name->text().trimmed().isEmpty()) {
        QMessageBox::warning(this, QStringLiteral("auto-resolution"), QStringLiteral("Give the rule a name first."));
        return;
    }

    QJsonObject payload = rule_json();
    payload.insert(QStringLiteral("original_name"), original_name);
    main->notify(QStringLiteral("duplicates_rule_save"), QString(), json_payload(payload));
    close();
}

HydrusDuplicatesRulesDialog::HydrusDuplicatesRulesDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution"));
    setObjectName(QStringLiteral("hydrus_duplicates_rules_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(980, 540);

    auto *layout = new QVBoxLayout(this);

    enabled = new QCheckBox(QStringLiteral("apply these rules automatically when a duplicates page refreshes"), this);
    layout->addWidget(enabled);

    auto *help = new QLabel(
        QStringLiteral("Rules are tested in alphabetical order; the first matching rule decides the pair. "
                       "Semi-automatic rules queue matches for review. Fully automatic rules apply immediately. "
                       "Run a preview before trusting a new rule."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    table = new QTableWidget(this);
    table->setColumnCount(5);
    table->setHorizontalHeaderLabels({
        QStringLiteral("name"),
        QStringLiteral("operation"),
        QStringLiteral("search"),
        QStringLiteral("comparison"),
        QStringLiteral("action")
    });
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    table->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    table->horizontalHeader()->setSectionResizeMode(4, QHeaderView::ResizeToContents);
    table->setSelectionBehavior(QAbstractItemView::SelectRows);
    table->setSelectionMode(QAbstractItemView::SingleSelection);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->verticalHeader()->setVisible(false);
    layout->addWidget(table, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add rule..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *duplicate = new QPushButton(QStringLiteral("duplicate"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    auto *review = new QPushButton(QStringLiteral("review pending..."), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(duplicate);
    buttons->addWidget(remove);
    buttons->addWidget(review);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(enabled, &QCheckBox::clicked, this, [this](bool) {
        main->notify(QStringLiteral("duplicates_auto_toggle"));
    });
    QObject::connect(add, &QPushButton::clicked, this, [this]() { add_rule(); });
    QObject::connect(edit, &QPushButton::clicked, this, [this]() { edit_selected(); });
    QObject::connect(duplicate, &QPushButton::clicked, this, [this]() { duplicate_selected(); });
    QObject::connect(remove, &QPushButton::clicked, this, [this]() { remove_selected(); });
    QObject::connect(review, &QPushButton::clicked, this, [this]() { open_review(); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);
    QObject::connect(table, &QTableWidget::cellDoubleClicked, this, [this](int, int) { edit_selected(); });

    set_state(state);

    const QString edit_key = qEnvironmentVariable("HYDRUS_QT_DUPLICATES_RULE_EDIT");

    if (!edit_key.isEmpty()) {
        const QString key = edit_key;
        QTimer::singleShot(150, this, [this, key]() {
            if (key == QStringLiteral("new")) {
                add_rule();
                return;
            }

            for (int row = 0; row < table->rowCount(); ++row) {
                if (table->item(row, 0) != nullptr
                    && table->item(row, 0)->text().compare(key, Qt::CaseInsensitive) == 0) {
                    table->selectRow(row);
                    edit_selected();
                    break;
                }
            }
        });
    }
}

void HydrusDuplicatesRulesDialog::set_state(const QJsonObject &state) {
    data = state;

    QJsonArray rules = state.value(QStringLiteral("auto_rules")).toArray();
    if (rules.isEmpty()) {
        rules = state.value(QStringLiteral("rules")).toArray();
    }
    const int previous_row = table->currentRow();
    table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 5; ++column) {
            if (table->item(index, column) == nullptr) {
                table->setItem(index, column, new QTableWidgetItem());
            }
        }

        QString operation_label = rule.value(QStringLiteral("operation_label")).toString();
        if (rule.value(QStringLiteral("paused")).toBool()) {
            operation_label += QStringLiteral(" - paused");
        }

        table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());
        table->item(index, 0)->setData(
            Qt::UserRole,
            QString::fromUtf8(QJsonDocument(rule).toJson(QJsonDocument::Compact)));
        table->item(index, 1)->setText(operation_label);
        table->item(index, 2)->setText(rule.value(QStringLiteral("search_summary")).toString());
        table->item(index, 3)->setText(rule.value(QStringLiteral("selector_summary")).toString());
        table->item(index, 4)->setText(rule.value(QStringLiteral("action_summary")).toString());
    }

    if (previous_row >= 0 && previous_row < table->rowCount()) {
        table->selectRow(previous_row);
    }

    QSignalBlocker blocker(enabled);
    enabled->setChecked(state.value(QStringLiteral("enabled")).toBool());

    const int pending = state.value(QStringLiteral("pending")).toInt(0);
    setToolTip(pending > 0 ? QStringLiteral("%1 pair(s) are waiting for review").arg(pending) : QString());

    table->resizeRowsToContents();
}

void HydrusDuplicatesRulesDialog::set_preview(const QJsonObject &preview) {
    if (rule_editor != nullptr) {
        rule_editor->set_preview(preview);
    }
}

void HydrusDuplicatesRulesDialog::add_rule() {
    QJsonObject rule;
    rule.insert(QStringLiteral("name"), QStringLiteral("new rule"));
    rule.insert(QStringLiteral("paused"), false);
    rule.insert(QStringLiteral("operation_mode"), QStringLiteral("semi-automatic"));
    rule.insert(QStringLiteral("max_pending_pairs"), 500);
    rule.insert(QStringLiteral("search_query"), QString());
    rule.insert(QStringLiteral("search_mode"), QStringLiteral("both"));
    rule.insert(QStringLiteral("max_distance"), 0);
    rule.insert(QStringLiteral("action"), QStringLiteral("better"));
    rule.insert(QStringLiteral("delete_a"), false);
    rule.insert(QStringLiteral("delete_b"), false);

    QJsonObject relative;
    relative.insert(QStringLiteral("kind"), QStringLiteral("relative"));
    relative.insert(QStringLiteral("property"), QStringLiteral("num_pixels"));
    relative.insert(QStringLiteral("operator"), QStringLiteral(">"));
    relative.insert(QStringLiteral("multiplier"), 1.0);
    relative.insert(QStringLiteral("delta"), 0.0);

    QJsonArray selector;
    selector.append(relative);
    rule.insert(QStringLiteral("pair_selector"), selector);

    edit_rule(rule);
}

void HydrusDuplicatesRulesDialog::edit_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error == QJsonParseError::NoError && document.isObject()) {
        edit_rule(document.object());
    }
}

void HydrusDuplicatesRulesDialog::duplicate_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(
        table->item(row, 0)->data(Qt::UserRole).toString().toUtf8(), &error);

    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return;
    }

    QJsonObject copy = document.object();
    copy.insert(QStringLiteral("name"), copy.value(QStringLiteral("name")).toString() + QStringLiteral(" copy"));
    copy.insert(QStringLiteral("paused"), true);
    edit_rule(copy);
}

void HydrusDuplicatesRulesDialog::remove_selected() {
    const int row = table->currentRow();

    if (row < 0 || table->item(row, 0) == nullptr) {
        return;
    }

    const QString name = table->item(row, 0)->text();

    if (QMessageBox::question(this, QStringLiteral("delete auto-resolution rule"),
                              QStringLiteral("Delete rule \"%1\" and its pending review pairs?").arg(name))
        != QMessageBox::Yes) {
        return;
    }

    main->notify(QStringLiteral("duplicates_rule_remove"), QString(), name);
}

void HydrusDuplicatesRulesDialog::open_review() {
    main->notify(QStringLiteral("duplicates_review"));
}

void HydrusDuplicatesRulesDialog::edit_rule(const QJsonObject &rule) {
    if (rule_editor != nullptr) {
        rule_editor->close();
        rule_editor = nullptr;
    }

    rule_editor = new HydrusAutoResolutionRuleDialog(main, this, rule, data, this);
    QObject::connect(rule_editor, &QObject::destroyed, this, [this]() { rule_editor = nullptr; });
    rule_editor->show();
    rule_editor->raise();
    rule_editor->activateWindow();
}

HydrusDuplicateReviewDialog::HydrusDuplicateReviewDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate auto-resolution review"));
    setObjectName(QStringLiteral("hydrus_duplicate_review_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(900, 520);

    auto *layout = new QVBoxLayout(this);

    status = new QLabel(this);
    status->setWordWrap(true);
    layout->addWidget(status);

    tabs = new QTabWidget(this);

    auto make_table = [this](const QStringList &headers) {
        auto *table = new QTableWidget(this);
        table->setColumnCount(headers.size());
        table->setHorizontalHeaderLabels(headers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::ExtendedSelection);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->verticalHeader()->setVisible(false);
        table->horizontalHeader()->setStretchLastSection(true);
        table->setIconSize(QSize(110, 90));

        for (int column = 0; column < 2 && column < table->columnCount(); ++column) {
            table->horizontalHeader()->setSectionResizeMode(column, QHeaderView::Fixed);
            table->setColumnWidth(column, 116);
        }

        return table;
    };

    pending = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("distance"),
        QStringLiteral("rule"),
        QStringLiteral("decision")
    });
    denied = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("rule")
    });
    actioned = make_table({
        QStringLiteral("A"),
        QStringLiteral("B"),
        QStringLiteral("decision"),
        QStringLiteral("rule"),
        QStringLiteral("denied")
    });

    install_pair_list_actions(pending, main);
    install_pair_list_actions(denied, main);
    install_pair_list_actions(actioned, main);

    tabs->addTab(pending, QStringLiteral("pending"));
    tabs->addTab(denied, QStringLiteral("denied"));
    tabs->addTab(actioned, QStringLiteral("taken"));
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *approve_selected = new QPushButton(QStringLiteral("approve selected"), this);
    auto *deny_selected = new QPushButton(QStringLiteral("deny selected"), this);
    auto *approve_all = new QPushButton(QStringLiteral("approve all"), this);
    auto *deny_all = new QPushButton(QStringLiteral("deny all"), this);
    auto *close = new QPushButton(QStringLiteral("close"), this);
    buttons->addWidget(approve_selected);
    buttons->addWidget(deny_selected);
    buttons->addWidget(approve_all);
    buttons->addWidget(deny_all);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addLayout(buttons);

    QObject::connect(approve_selected, &QPushButton::clicked, this, [this]() { apply_selected(true); });
    QObject::connect(deny_selected, &QPushButton::clicked, this, [this]() { apply_selected(false); });
    QObject::connect(approve_all, &QPushButton::clicked, this, [this]() { apply_all(true); });
    QObject::connect(deny_all, &QPushButton::clicked, this, [this]() { apply_all(false); });
    QObject::connect(close, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);
}

void HydrusDuplicateReviewDialog::set_state(const QJsonObject &state) {
    status->setText(state.value(QStringLiteral("status")).toString());

    fill_pair_table(
        pending,
        state.value(QStringLiteral("pending")).toArray(),
        2,
        {
            QStringLiteral("distance"),
            QStringLiteral("rule"),
            QStringLiteral("decision"),
        });
    fill_pair_table(
        denied,
        state.value(QStringLiteral("denied")).toArray(),
        2,
        {QStringLiteral("rule")});
    fill_pair_table(
        actioned,
        state.value(QStringLiteral("actioned")).toArray(),
        2,
        {
            QStringLiteral("decision"),
            QStringLiteral("rule"),
            QStringLiteral("banned"),
        });
}

QList<int> HydrusDuplicateReviewDialog::selected_pending_rows() const {
    QList<int> rows;

    for (const QModelIndex &index : pending->selectionModel()->selectedRows()) {
        if (!rows.contains(index.row())) {
            rows.append(index.row());
        }
    }

    std::sort(rows.begin(), rows.end());

    return rows;
}

void HydrusDuplicateReviewDialog::apply_selected(bool approve) {
    const QList<int> rows = selected_pending_rows();

    if (rows.isEmpty()) {
        QMessageBox::information(this, QStringLiteral("duplicate review"),
                                 QStringLiteral("Select at least one pending pair first."));
        return;
    }

    QJsonArray indices;
    for (int row : rows) {
        indices.append(row);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("indices"), indices);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}

void HydrusDuplicateReviewDialog::apply_all(bool approve) {
    QJsonObject payload;
    payload.insert(QStringLiteral("all"), true);
    payload.insert(QStringLiteral("approve"), approve);
    main->notify(QStringLiteral("duplicates_review_apply"), QString(), json_payload(payload));
}

HydrusDuplicateMergeOptionsDialog::HydrusDuplicateMergeOptionsDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("duplicate merge options"));
    setObjectName(QStringLiteral("hydrus_duplicate_merge_options_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(760, 620);

    data = state;

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose what a duplicate merge copies from the worse file. Services that are not "
                       "listed are left untouched. \"This is better\" keeps one file and can move or copy "
                       "metadata; \"same quality\" merges both files in the chosen direction."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *tabs = new QTabWidget(this);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("this is better"), better);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("same quality"), same);
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *save_button = new QPushButton(QStringLiteral("save"), this);
    auto *close_button = new QPushButton(QStringLiteral("close"), this);
    buttons->addStretch(1);
    buttons->addWidget(save_button);
    buttons->addWidget(close_button);
    layout->addLayout(buttons);

    connect(save_button, &QPushButton::clicked, this, [this]() { save(); });
    connect(close_button, &QPushButton::clicked, this, &QDialog::close);

    set_state(state);

    if (qEnvironmentVariable("HYDRUS_QT_DUPLICATES_MERGE_TAB") == QStringLiteral("same")) {
        tabs->setCurrentIndex(1);
    }
}

void HydrusDuplicateMergeOptionsDialog::set_state(const QJsonObject &state) {
    data = state;

    fill_duplicate_merge_page(better, data, data.value(QStringLiteral("better")).toObject());
    fill_duplicate_merge_page(same, data, data.value(QStringLiteral("same")).toObject());
}

void build_duplicate_merge_page(
    QWidget *parent,
    QTabWidget *tabs,
    const QJsonObject &data,
    const QString &title,
    DuplicateMergePage &page) {
    auto *widget = new QWidget(parent);
    auto *layout = new QVBoxLayout(widget);

    auto *form = new QFormLayout();

    page.archive = new QComboBox(widget);
    page.modified_date = new QComboBox(widget);
    page.urls = new QComboBox(widget);
    page.notes = new QComboBox(widget);

    for (const QJsonValue &action : data.value(QStringLiteral("archive_actions")).toArray()) {
        const QJsonObject entry = action.toObject();
        page.archive->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
    }

    for (QComboBox *combo : {page.modified_date, page.urls, page.notes}) {
        for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
            const QJsonObject entry = action.toObject();
            const QString value = entry.value(QStringLiteral("value")).toString();

            if (combo == page.modified_date && (value == QStringLiteral("move") || value == QStringLiteral("add_move"))) {
                continue;
            }

            combo->addItem(entry.value(QStringLiteral("label")).toString(), value);
        }
    }

    form->addRow(QStringLiteral("sync archived status:"), page.archive);
    form->addRow(QStringLiteral("sync file modified time:"), page.modified_date);
    form->addRow(QStringLiteral("sync known urls:"), page.urls);
    form->addRow(QStringLiteral("sync notes:"), page.notes);
    layout->addLayout(form);

    page.tags = new QTableWidget(widget);
    page.tags->setColumnCount(2);
    page.tags->setHorizontalHeaderLabels({QStringLiteral("tag service"), QStringLiteral("action")});
    page.tags->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.tags->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.tags->verticalHeader()->setVisible(false);
    page.tags->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.tags->setSelectionMode(QAbstractItemView::NoSelection);

    page.ratings = new QTableWidget(widget);
    page.ratings->setColumnCount(2);
    page.ratings->setHorizontalHeaderLabels({QStringLiteral("rating service"), QStringLiteral("action")});
    page.ratings->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    page.ratings->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    page.ratings->verticalHeader()->setVisible(false);
    page.ratings->setEditTriggers(QAbstractItemView::NoEditTriggers);
    page.ratings->setSelectionMode(QAbstractItemView::NoSelection);

    layout->addWidget(new QLabel(QStringLiteral("tag services"), widget));
    layout->addWidget(page.tags, 1);
    layout->addWidget(new QLabel(QStringLiteral("rating services"), widget));
    layout->addWidget(page.ratings, 1);

    tabs->addTab(widget, title);
}

void fill_duplicate_merge_page(
    DuplicateMergePage &page,
    const QJsonObject &data,
    const QJsonObject &options) {
    auto select_value = [](QComboBox *combo, const QString &value) {
        const int index = combo->findData(value);

        if (index >= 0) {
            combo->setCurrentIndex(index);
        }
    };

    select_value(page.archive, options.value(QStringLiteral("sync_archive_action")).toString());
    select_value(page.modified_date, options.value(QStringLiteral("sync_file_modified_date_action")).toString());
    select_value(page.urls, options.value(QStringLiteral("sync_urls_action")).toString());
    select_value(page.notes, options.value(QStringLiteral("sync_notes_action")).toString());

    auto fill_table = [&data](QTableWidget *table, const QJsonArray &services, const QJsonObject &actions) {
        table->setRowCount(0);

        for (const QJsonValue &service_value : services) {
            const QString service = service_value.toString();
            const int row = table->rowCount();
            table->insertRow(row);
            table->setItem(row, 0, new QTableWidgetItem(service));

            auto *combo = new QComboBox(table);

            for (const QJsonValue &action : data.value(QStringLiteral("actions")).toArray()) {
                const QJsonObject entry = action.toObject();
                combo->addItem(entry.value(QStringLiteral("label")).toString(), entry.value(QStringLiteral("value")).toString());
            }

            const int index = combo->findData(actions.value(service).toString());

            if (index >= 0) {
                combo->setCurrentIndex(index);
            } else {
                const int none_index = combo->findData(QStringLiteral("none"));

                if (none_index >= 0) {
                    combo->setCurrentIndex(none_index);
                }
            }

            table->setCellWidget(row, 1, combo);
        }
    };

    fill_table(
        page.tags,
        data.value(QStringLiteral("tag_services")).toArray(),
        options.value(QStringLiteral("tag_service_actions")).toObject());
    fill_table(
        page.ratings,
        data.value(QStringLiteral("rating_services")).toArray(),
        options.value(QStringLiteral("rating_service_actions")).toObject());
}

QJsonObject read_duplicate_merge_page(const DuplicateMergePage &page) {
    auto read_table = [](QTableWidget *table) {
        QJsonObject values;

        for (int row = 0; row < table->rowCount(); ++row) {
            const QTableWidgetItem *service = table->item(row, 0);
            auto *combo = qobject_cast<QComboBox *>(table->cellWidget(row, 1));

            if (service == nullptr || combo == nullptr) {
                continue;
            }

            const QString action = combo->currentData().toString();

            if (action != QStringLiteral("none")) {
                values.insert(service->text(), action);
            }
        }

        return values;
    };

    QJsonObject options;
    options.insert(QStringLiteral("sync_archive_action"), page.archive->currentData().toString());
    options.insert(QStringLiteral("sync_file_modified_date_action"), page.modified_date->currentData().toString());
    options.insert(QStringLiteral("sync_urls_action"), page.urls->currentData().toString());
    options.insert(QStringLiteral("sync_notes_action"), page.notes->currentData().toString());
    options.insert(QStringLiteral("tag_service_actions"), read_table(page.tags));
    options.insert(QStringLiteral("rating_service_actions"), read_table(page.ratings));

    return options;
}

void HydrusDuplicateMergeOptionsDialog::save() {
    for (const auto &[key, page] : std::initializer_list<std::pair<QString, const DuplicateMergePage *>>{
             {QStringLiteral("better"), &better}, {QStringLiteral("same"), &same}}) {
        QJsonObject payload;
        payload.insert(QStringLiteral("action"), key);
        payload.insert(QStringLiteral("options"), read_duplicate_merge_page(*page));
        main->notify(QStringLiteral("duplicates_merge_options"), QString(), json_payload(payload));
    }

    close();
}

HydrusDuplicateCustomActionDialog::HydrusDuplicateCustomActionDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("custom duplicate action"));
    setObjectName(QStringLiteral("hydrus_duplicate_custom_action_dialog"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(800, 700);

    data = state;

    auto *layout = new QVBoxLayout(this);

    auto *help = new QLabel(
        QStringLiteral("Choose one action and customise the merge and delete options for this pair only. "
                       "Saved merge options are not changed."),
        this);
    help->setWordWrap(true);
    layout->addWidget(help);

    const QJsonObject pair = data.value(QStringLiteral("pair")).toObject();

    if (!pair.isEmpty()) {
        const QJsonObject a = pair.value(QStringLiteral("a")).toObject();
        const QJsonObject b = pair.value(QStringLiteral("b")).toObject();

        auto *summary = new QLabel(
            QStringLiteral("A: %1  %2\nB: %3  %4")
                .arg(a.value(QStringLiteral("hash")).toString().left(16),
                     a.value(QStringLiteral("info")).toString(),
                     b.value(QStringLiteral("hash")).toString().left(16),
                     b.value(QStringLiteral("info")).toString()),
            this);
        summary->setWordWrap(true);
        summary->setStyleSheet(QStringLiteral("font-family: monospace;"));
        layout->addWidget(summary);
    }

    auto *form = new QFormLayout();

    action = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("duplicate_types")).toArray()) {
        const QJsonObject object = entry.toObject();
        action->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    better_side = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("better_sides")).toArray()) {
        const QJsonObject object = entry.toObject();
        better_side->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    delete_mode = new QComboBox(this);
    for (const QJsonValue &entry : data.value(QStringLiteral("delete_modes")).toArray()) {
        const QJsonObject object = entry.toObject();
        delete_mode->addItem(object.value(QStringLiteral("label")).toString(), object.value(QStringLiteral("value")).toString());
    }

    form->addRow(QStringLiteral("action:"), action);
    form->addRow(QStringLiteral("better file:"), better_side);
    form->addRow(QStringLiteral("delete:"), delete_mode);
    layout->addLayout(form);

    tabs = new QTabWidget(this);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("this is better"), better);
    build_duplicate_merge_page(this, tabs, data, QStringLiteral("same quality"), same);
    fill_duplicate_merge_page(better, data, data.value(QStringLiteral("better")).toObject());
    fill_duplicate_merge_page(same, data, data.value(QStringLiteral("same")).toObject());
    layout->addWidget(tabs, 1);

    auto *buttons = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *apply_button = new QPushButton(QStringLiteral("apply"), this);
    apply_button->setDefault(true);
    buttons->addStretch(1);
    buttons->addWidget(cancel);
    buttons->addWidget(apply_button);
    layout->addLayout(buttons);

    auto update_controls = [this]() {
        const QString kind = action->currentData().toString();
        const bool better_action = kind == QStringLiteral("better");

        better_side->setEnabled(better_action);
        tabs->setTabEnabled(0, better_action);
        tabs->setTabEnabled(1, kind == QStringLiteral("same"));

        if (better_action) {
            tabs->setCurrentIndex(0);
        } else if (kind == QStringLiteral("same")) {
            tabs->setCurrentIndex(1);
        }
    };

    connect(action, &QComboBox::currentIndexChanged, this, [update_controls](int) { update_controls(); });
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(apply_button, &QPushButton::clicked, this, [this]() { apply(); });

    update_controls();
}

void HydrusDuplicateCustomActionDialog::apply() {
    QJsonObject payload;
    payload.insert(QStringLiteral("action"), action->currentData().toString());
    payload.insert(QStringLiteral("better_side"), better_side->currentData().toString());
    payload.insert(QStringLiteral("delete"), delete_mode->currentData().toString());
    payload.insert(QStringLiteral("better"), read_duplicate_merge_page(better));
    payload.insert(QStringLiteral("same"), read_duplicate_merge_page(same));

    main->notify(
        QStringLiteral("duplicates_custom"),
        data.value(QStringLiteral("page_id")).toString(),
        json_payload(payload));

    close();
}

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


HydrusMainWindow::HydrusMainWindow(QWidget *parent) : QMainWindow(parent) {
    active_notebook = QStringLiteral("root");
    setAcceptDrops(true);
}

void HydrusMainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
    }
}

void HydrusMainWindow::dropEvent(QDropEvent *event) {
    QStringList paths;

    for (const QUrl &url : event->mimeData()->urls()) {
        if (url.isLocalFile()) {
            paths.append(url.toLocalFile());
        }
    }

    if (paths.isEmpty()) {
        return;
    }

    QJsonArray array;

    for (const QString &path : paths) {
        array.append(path);
    }

    notify(QStringLiteral("local_import_open_paths"), QString(), QString::fromUtf8(QJsonDocument(array).toJson(QJsonDocument::Compact)));
    event->acceptProposedAction();
}

void HydrusMainWindow::closeEvent(QCloseEvent *event) {
    if (tick_timer != nullptr) {
        tick_timer->stop();
    }

    QSettings settings;
    settings.setValue(QStringLiteral("main/geometry"), saveGeometry());
    settings.setValue(QStringLiteral("main/state"), saveState());
    settings.sync();

    if (tray != nullptr) {
        tray->hide();
    }

    if (services_dialog != nullptr) {
        services_dialog->close();
    }

    if (network_dialog != nullptr) {
        network_dialog->close();
    }

    QMainWindow::closeEvent(event);
}

void HydrusMainWindow::notify(const QString &action, const QString &target, const QString &payload) {
    QByteArray action_bytes = action.toUtf8();
    QByteArray target_bytes = target.toUtf8();
    QByteArray payload_bytes = payload.toUtf8();

    app_action(
        *this,
        rust::Str(action_bytes.constData(), action_bytes.size()),
        rust::Str(target_bytes.constData(), target_bytes.size()),
        rust::Str(payload_bytes.constData(), payload_bytes.size()));
}

void HydrusMainWindow::build_shell() {
    if (root != nullptr) {
        return;
    }

    setStyleSheet(QString());

    root = new HydrusNotebook(this, QStringLiteral("root"), this);
    setCentralWidget(root);

    auto text_width = [this](int chars) {
        return fontMetrics().horizontalAdvance(QString(chars, QLatin1Char('0')));
    };

    status_media = new QLabel(this);
    status_bandwidth = new QLabel(this);
    status_bandwidth->setFixedWidth(text_width(17));
    status_idle = new QLabel(this);
    status_idle->setFixedWidth(text_width(6));
    status_hydrus_busy = new QLabel(this);
    status_hydrus_busy->setFixedWidth(text_width(11));
    status_cpu_busy = new QLabel(this);
    status_cpu_busy->setFixedWidth(text_width(13));
    status_db = new QLabel(this);
    status_db->setFixedWidth(text_width(14));

    statusBar()->addWidget(status_media, 1);
    statusBar()->addWidget(status_bandwidth);
    statusBar()->addWidget(status_idle);
    statusBar()->addWidget(status_hydrus_busy);
    statusBar()->addWidget(status_cpu_busy);
    statusBar()->addWidget(status_db);
    statusBar()->setSizeGripEnabled(true);

    if (QSystemTrayIcon::isSystemTrayAvailable()) {
        tray = new QSystemTrayIcon(style()->standardIcon(QStyle::SP_ComputerIcon), this);
        tray->setToolTip(QStringLiteral("Hydrus Client"));

        auto *tray_menu = new QMenu(this);
        auto *show_action = tray_menu->addAction(QStringLiteral("show/hide"));
        auto *tray_exit = tray_menu->addAction(QStringLiteral("exit"));

        connect(show_action, &QAction::triggered, this, [this]() {
            if (isVisible()) {
                hide();
            } else {
                show();
                raise();
                activateWindow();
            }
        });
        connect(tray_exit, &QAction::triggered, this, [this]() { notify(QStringLiteral("exit")); });
        connect(tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
            if (reason == QSystemTrayIcon::Trigger) {
                if (isVisible()) {
                    hide();
                } else {
                    show();
                    raise();
                    activateWindow();
                }
            }
        });

        tray->setContextMenu(tray_menu);
        tray->show();
    }

    build_shortcuts();

    resize(1280, 800);
    setWindowTitle(QStringLiteral("Hydrus Client"));

    QSettings settings;
    const QByteArray geometry = settings.value(QStringLiteral("main/geometry")).toByteArray();
    const QByteArray state = settings.value(QStringLiteral("main/state")).toByteArray();

    if (!geometry.isEmpty()) {
        restoreGeometry(geometry);
    }

    if (!state.isEmpty()) {
        restoreState(state);
    }
}

void show_command_palette(HydrusMainWindow *main);

void HydrusMainWindow::build_shortcuts() {
    auto *new_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+T")), this);
    connect(new_page, &QShortcut::activated, this, [this]() {
        show_page_chooser(active_notebook.isEmpty() ? QStringLiteral("root") : active_notebook);
    });

    auto *close_page = new QShortcut(QKeySequence(QStringLiteral("Ctrl+W")), this);
    connect(close_page, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("close_page"), active_page);
        }
    });

    auto *refresh = new QShortcut(QKeySequence(QStringLiteral("Ctrl+R")), this);
    connect(refresh, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("refresh_page"), active_page);
        }
    });

    auto *undo = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Z")), this);
    connect(undo, &QShortcut::activated, this, [this]() { notify(QStringLiteral("undo")); });

    auto *back = new QShortcut(QKeySequence(QStringLiteral("Alt+Left")), this);
    connect(back, &QShortcut::activated, this, [this]() { notify(QStringLiteral("history_back")); });

    auto *forward = new QShortcut(QKeySequence(QStringLiteral("Alt+Right")), this);
    connect(forward, &QShortcut::activated, this, [this]() { notify(QStringLiteral("history_forward")); });

    auto *page_manager = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+P")), this);
    connect(page_manager, &QShortcut::activated, this, [this]() { notify(QStringLiteral("page_manager")); });

    auto *palette = new QShortcut(QKeySequence(QStringLiteral("Ctrl+K")), this);
    connect(palette, &QShortcut::activated, this, [this]() {
        show_command_palette(this);
    });

    auto *duplicate = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+T")), this);
    connect(duplicate, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("duplicate_page"), active_page);
        }
    });

    auto *import = new QShortcut(QKeySequence(QStringLiteral("Ctrl+I")), this);
    connect(import, &QShortcut::activated, this, [this]() { notify(QStringLiteral("import_files")); });

    auto *import_url = new QShortcut(QKeySequence(QStringLiteral("Ctrl+U")), this);
    connect(import_url, &QShortcut::activated, this, [this]() { notify(QStringLiteral("import_url")); });

    auto *import_folders_shortcut = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+I")), this);
    connect(import_folders_shortcut, &QShortcut::activated, this, [this]() {
        notify(QStringLiteral("new_import_folders"));
    });

    auto *options = new QShortcut(QKeySequence(QStringLiteral("Ctrl+P")), this);
    connect(options, &QShortcut::activated, this, [this]() { notify(QStringLiteral("options")); });

    auto *manage_tags = new QShortcut(QKeySequence(QStringLiteral("Ctrl+M")), this);
    manage_tags->setContext(Qt::ApplicationShortcut);
    connect(manage_tags, &QShortcut::activated, this, [this]() {
        notify(QStringLiteral("manage_tags_active"));
    });

    auto *archive = new QShortcut(QKeySequence(QStringLiteral("F7")), this);
    connect(archive, &QShortcut::activated, this, [this]() { notify(QStringLiteral("archive_active")); });

    auto *inbox = new QShortcut(QKeySequence(QStringLiteral("F8")), this);
    connect(inbox, &QShortcut::activated, this, [this]() { notify(QStringLiteral("inbox_active")); });

    auto *refresh_f5 = new QShortcut(QKeySequence(QStringLiteral("F5")), this);
    connect(refresh_f5, &QShortcut::activated, this, [this]() {
        if (!active_page.isEmpty()) {
            notify(QStringLiteral("refresh_page"), active_page);
        }
    });

    auto *quit = new QShortcut(QKeySequence(QStringLiteral("Ctrl+Q")), this);
    connect(quit, &QShortcut::activated, this, [this]() { notify(QStringLiteral("exit")); });
}

void HydrusMainWindow::set_status(const QJsonObject &status) {
    const QString media = status.value(QStringLiteral("media")).toString();
    const QString message = status.value(QStringLiteral("message")).toString();

    status_media->setText(media.isEmpty() ? message : media);
    status_media->setToolTip(message.isEmpty() ? media : message);
    status_bandwidth->setText(status.value(QStringLiteral("bandwidth")).toString());
    status_idle->setText(status.value(QStringLiteral("idle")).toString());
    status_hydrus_busy->setText(status.value(QStringLiteral("hydrus_busy")).toString());
    status_cpu_busy->setText(status.value(QStringLiteral("cpu_busy")).toString());
    status_db->setText(status.value(QStringLiteral("db")).toString());
}

void HydrusMainWindow::set_menus(const QJsonArray &menus) {
    menuBar()->clear();

    for (const QJsonValue &value : menus) {
        const QJsonObject menu_object = value.toObject();
        auto *menu = menuBar()->addMenu(menu_object.value(QStringLiteral("title")).toString());
        menu->setToolTipsVisible(true);

        std::function<void(QMenu *, const QJsonArray &)> add_items;

        add_items = [this, &add_items](QMenu *menu_target, const QJsonArray &items) {
            for (const QJsonValue &item_value : items) {
                const QJsonObject item = item_value.toObject();

                if (item.value(QStringLiteral("separator")).toBool()) {
                    menu_target->addSeparator();
                    continue;
                }

                const QJsonArray children = item.value(QStringLiteral("items")).toArray();

                if (!children.isEmpty()) {
                    auto *submenu = menu_target->addMenu(item.value(QStringLiteral("label")).toString());
                    submenu->setToolTipsVisible(true);
                    add_items(submenu, children);
                    continue;
                }

                const QString label = item.value(QStringLiteral("label")).toString();
                const QString action_id = item.value(QStringLiteral("action")).toString();
                const QString item_target = item.value(QStringLiteral("target")).toString();
                const QString payload = item.value(QStringLiteral("payload")).toString();

                if (label.isEmpty()) {
                    continue;
                }

                auto *action = menu_target->addAction(label);
                action->setEnabled(item.value(QStringLiteral("enabled")).toBool(true));

                if (item.value(QStringLiteral("checkable")).toBool(false)) {
                    action->setCheckable(true);
                    action->setChecked(item.value(QStringLiteral("checked")).toBool(false));
                }

                const QString tooltip = item.value(QStringLiteral("description")).toString();

                if (!tooltip.isEmpty()) {
                    action->setToolTip(tooltip);
                }

                connect(action, &QAction::triggered, this, [this, action_id, item_target, payload]() {
                    if (!action_id.isEmpty()) {
                        notify(action_id, item_target, payload);
                    }
                });
            }
        };

        add_items(menu, menu_object.value(QStringLiteral("items")).toArray());
    }
}

void HydrusMainWindow::clear_notebook(HydrusNotebook *notebook) {
    notebook->clear_pages();
}

void HydrusMainWindow::set_tag_suggestions(const QStringList &suggestions) {
    tag_suggestions = suggestions;

    for (HydrusQueryPage *page : pages) {
        page->set_suggestions(tag_suggestions);
    }
}

const QStringList &HydrusMainWindow::suggestions() const {
    return tag_suggestions;
}

void HydrusMainWindow::show_autocomplete(const QJsonObject &state) {
    const QString page_id = state.value(QStringLiteral("page_id")).toString();
    HydrusQueryPage *page = pages.value(page_id, nullptr);

    if (page != nullptr) {
        page->show_autocomplete(state);
    }
}

void HydrusMainWindow::build_pages(const QJsonObject &object) {
    if (root == nullptr) {
        return;
    }

    building = true;

    pages.clear();
    downloaders.clear();
    subscriptions.clear();
    duplicates.clear();
    import_folders.clear();
    export_folders.clear();
    notebooks.clear();
    notebooks.insert(QStringLiteral("root"), root);

    clear_notebook(root);

    const QJsonObject root_object = object.value(QStringLiteral("root")).toObject();
    populate_notebook(root, root_object);

    QString current = root->current_page_id();
    QString parent = QStringLiteral("root");

    while (HydrusNotebook *nested = notebooks.value(current, nullptr)) {
        parent = current;
        current = nested->current_page_id();
    }

    active_page = current;
    active_notebook = parent;

    building = false;
}

void HydrusMainWindow::populate_notebook(HydrusNotebook *notebook, const QJsonObject &object) {
    const QJsonArray children = object.value(QStringLiteral("children")).toArray();

    for (const QJsonValue &value : children) {
        const QJsonObject child = value.toObject();
        const QString kind = child.value(QStringLiteral("kind")).toString();
        const QString page_id = child.value(QStringLiteral("id")).toString();
        const QString title = child.value(QStringLiteral("title")).toString();

        if (kind == QStringLiteral("notebook")) {
            auto *nested = new HydrusNotebook(this, page_id, notebook);
            notebooks.insert(page_id, nested);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page of pages") : title, nested);
            populate_notebook(nested, child);
        } else if (kind == QStringLiteral("query")) {
            auto *page = new HydrusQueryPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            pages.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("files") : title, page);
        } else if (kind == QStringLiteral("downloader")) {
            auto *page = new HydrusDownloaderPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            downloaders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("download") : title, page);
        } else if (kind == QStringLiteral("subscriptions")) {
            auto *page = new HydrusSubscriptionsPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            subscriptions.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("subscriptions") : title, page);
        } else if (kind == QStringLiteral("duplicates")) {
            auto *page = new HydrusDuplicatesPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            duplicates.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("duplicates") : title, page);
        } else if (kind == QStringLiteral("import_folders")) {
            auto *page = new HydrusImportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            import_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("import folders") : title, page);
        } else if (kind == QStringLiteral("export_folders")) {
            auto *page = new HydrusExportFoldersPage(this, page_id, child.value(QStringLiteral("state")).toObject());
            export_folders.insert(page_id, page);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("export folders") : title, page);
        } else {
            const QString message = child.value(QStringLiteral("state")).toObject().value(QStringLiteral("message")).toString();
            auto *placeholder = new QWidget(notebook);
            auto *layout = new QVBoxLayout(placeholder);
            auto *label = new QLabel(message.isEmpty() ? QStringLiteral("Unknown page type.") : message, placeholder);
            label->setAlignment(Qt::AlignCenter);
            label->setWordWrap(true);
            layout->addWidget(label);
            notebook->add_page_widget(page_id, title.isEmpty() ? QStringLiteral("page") : title, placeholder);
        }
    }

    notebook->finalize_notebook(object.value(QStringLiteral("current")).toString());
}

HydrusQueryPage *HydrusMainWindow::find_page(const QString &page_id) const {
    return pages.value(page_id, nullptr);
}

void HydrusMainWindow::update_page(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_page_progress(const QString &page_id, const QJsonObject &state) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->update_progress(state);
    }
}

void HydrusMainWindow::update_downloader(const QString &page_id, const QJsonObject &state) {
    if (HydrusDownloaderPage *page = downloaders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_subscriptions(const QString &page_id, const QJsonObject &state) {
    if (HydrusSubscriptionsPage *page = subscriptions.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_duplicates(const QString &page_id, const QJsonObject &state) {
    if (HydrusDuplicatesPage *page = duplicates.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusDuplicatesPage::update_auto_resolution_rules(const QJsonObject &state) {
    if (auto_rules_table == nullptr) {
        return;
    }

    QJsonArray rules = state.value(QStringLiteral("auto_rules")).toArray();
    if (rules.isEmpty()) {
        rules = state.value(QStringLiteral("rules")).toArray();
    }
    auto_rules_table->setRowCount(rules.size());

    for (int index = 0; index < rules.size(); ++index) {
        const QJsonObject rule = rules.at(index).toObject();

        for (int column = 0; column < 3; ++column) {
            if (auto_rules_table->item(index, column) == nullptr) {
                auto_rules_table->setItem(index, column, new QTableWidgetItem());
            }
        }

        auto_rules_table->item(index, 0)->setText(rule.value(QStringLiteral("name")).toString());

        QString progress = rule.value(QStringLiteral("search_summary")).toString();
        auto_rules_table->item(index, 1)->setText(progress);

        QString status = rule.value(QStringLiteral("paused")).toBool()
                             ? QStringLiteral("paused")
                             : QStringLiteral("running");
        auto_rules_table->item(index, 2)->setText(status);
    }

    if (auto_rules_status != nullptr) {
        if (rules.isEmpty()) {
            auto_rules_status->setText(QStringLiteral("no auto-resolution rules yet"));
            auto_rules_status->setVisible(true);
        } else {
            auto_rules_status->setVisible(false);
        }
    }

    auto_rules_table->resizeRowsToContents();
}


void HydrusMainWindow::update_import_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusImportFoldersPage *page = import_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::update_export_folders(const QString &page_id, const QJsonObject &state) {
    if (HydrusExportFoldersPage *page = export_folders.value(page_id, nullptr)) {
        page->update_state(state);
    }
}

void HydrusMainWindow::set_selection(
    const QString &page_id,
    const QString &hash,
    const QStringList &tags,
    const QString &preview_path,
    const QJsonObject &metadata) {
    if (HydrusQueryPage *page = find_page(page_id)) {
        page->set_selection(hash, tags, preview_path, metadata);
    }
}

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

void HydrusMainWindow::prompt_url() {
    bool ok = false;
    const QString url = QInputDialog::getText(
        this,
        QStringLiteral("import url"),
        QStringLiteral("file url"),
        QLineEdit::Normal,
        QString(),
        &ok);

    if (ok && !url.isEmpty()) {
        notify(QStringLiteral("download_url"), QString(), url);
    }
}

void HydrusMainWindow::open_viewer(const QJsonObject &state) {
    const QString viewer_id = state.value(QStringLiteral("viewer_id")).toString();

    if (viewer_id.isEmpty()) {
        return;
    }

    if (HydrusViewerWindow *existing = viewers.value(viewer_id, nullptr)) {
        existing->update_state(state);
        existing->raise();
        existing->activateWindow();
        return;
    }

    auto *viewer = new HydrusViewerWindow(this, state);
    viewer->setAttribute(Qt::WA_DeleteOnClose, true);
    viewers.insert(viewer_id, viewer);
    viewer->show();
}

void HydrusMainWindow::update_viewer(const QString &viewer_id, const QJsonObject &state) {
    if (HydrusViewerWindow *viewer = viewers.value(viewer_id, nullptr)) {
        viewer->update_state(state);
    }
}

void HydrusMainWindow::close_viewer(const QString &viewer_id) {
    if (HydrusViewerWindow *viewer = viewers.value(viewer_id, nullptr)) {
        viewers.remove(viewer_id);
        viewer->close();
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

// One tag from a directory/regex rule: an empty namespace means the value is
// added unnamespaced.
static QString join_filename_tag(const QString &tag_namespace, const QString &value) {
    const QString cleaned = value.trimmed();

    if (cleaned.isEmpty()) {
        return QString();
    }

    return tag_namespace.trimmed().isEmpty() ? cleaned : QStringLiteral("%1:%2").arg(tag_namespace.trimmed(), cleaned);
}

// Mirrors FilenameTaggingOptions::get_tags closely enough to preview rules
// while the dialog is open. The Rust side does the real cleaning and storage.
static QString filename_tagging_preview(const QJsonObject &options, const QString &path_text) {
    QStringList tags;

    for (const QJsonValue &value : options.value(QStringLiteral("tags_for_all")).toArray()) {
        tags << value.toString();
    }

    const QFileInfo info(path_text.trimmed());

    if (options.value(QStringLiteral("add_filename")).toBool() && !info.completeBaseName().isEmpty()) {
        tags << join_filename_tag(options.value(QStringLiteral("filename_namespace")).toString(), info.completeBaseName());
    }

    QStringList directories = QDir::fromNativeSeparators(info.absolutePath()).split(QLatin1Char('/'), Qt::SkipEmptyParts);

    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        const QJsonObject rule = options.value(QStringLiteral("directories")).toObject().value(key).toObject();

        if (!rule.value(QStringLiteral("enabled")).toBool()) {
            continue;
        }

        int index = key.toInt();
        const int resolved = index < 0 ? directories.size() + index : index;

        if (resolved < 0 || resolved >= directories.size()) {
            continue;
        }

        tags << join_filename_tag(rule.value(QStringLiteral("namespace")).toString(), directories.at(resolved));
    }

    auto collect = [&tags](const QString &pattern, const QString &tag_namespace, const QString &text) {
        const QRegularExpression regex(pattern);

        if (!regex.isValid()) {
            return;
        }

        QRegularExpressionMatchIterator iterator = regex.globalMatch(text);

        while (iterator.hasNext()) {
            const QRegularExpressionMatch match = iterator.next();

            if (regex.captureCount() > 0) {
                for (int group = 1; group <= regex.captureCount(); ++group) {
                    if (match.capturedStart(group) >= 0) {
                        tags << join_filename_tag(tag_namespace, match.captured(group));
                    }
                }
            } else {
                tags << join_filename_tag(tag_namespace, match.captured(0));
            }
        }
    };

    const QString path = QDir::fromNativeSeparators(path_text.trimmed());

    for (const QJsonValue &value : options.value(QStringLiteral("regexes")).toArray()) {
        collect(value.toString(), QString(), path);
    }

    for (const QJsonValue &value : options.value(QStringLiteral("quick_namespaces")).toArray()) {
        const QJsonObject rule = value.toObject();
        collect(rule.value(QStringLiteral("regex")).toString(), rule.value(QStringLiteral("namespace")).toString(), path);
    }

    QStringList cleaned;

    for (const QString &tag : tags) {
        const QString trimmed = tag.trimmed();

        if (!trimmed.isEmpty() && !cleaned.contains(trimmed, Qt::CaseInsensitive)) {
            cleaned << trimmed;
        }
    }

    cleaned.sort(Qt::CaseInsensitive);

    return cleaned.join(QStringLiteral(", "));
}

static QString filename_tagging_summary(const QJsonObject &options) {
    QStringList statements;

    const QJsonArray all_tags = options.value(QStringLiteral("tags_for_all")).toArray();

    if (!all_tags.isEmpty()) {
        statements << QStringLiteral("%1 tag(s) for all").arg(all_tags.size());
    }

    if (options.value(QStringLiteral("add_filename")).toBool()) {
        statements << QStringLiteral("filename as '%1'").arg(options.value(QStringLiteral("filename_namespace")).toString());
    }

    int directories = 0;
    const QJsonObject rules = options.value(QStringLiteral("directories")).toObject();

    for (const QJsonValue &rule : rules) {
        if (rule.toObject().value(QStringLiteral("enabled")).toBool()) {
            ++directories;
        }
    }

    if (directories > 0) {
        statements << QStringLiteral("%1 directory rule(s)").arg(directories);
    }

    const int quick = options.value(QStringLiteral("quick_namespaces")).toArray().size();

    if (quick > 0) {
        statements << QStringLiteral("%1 namespace regex(es)").arg(quick);
    }

    const int regexes = options.value(QStringLiteral("regexes")).toArray().size();

    if (regexes > 0) {
        statements << QStringLiteral("%1 regex(es)").arg(regexes);
    }

    return statements.isEmpty() ? QStringLiteral("no tags") : statements.join(QStringLiteral(", "));
}

static QJsonObject default_filename_tagging_options() {
    QJsonObject directories;

    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        directories.insert(key, QJsonObject{
            {QStringLiteral("enabled"), false},
            {QStringLiteral("namespace"), QString()},
        });
    }

    return QJsonObject{
        {QStringLiteral("tags_for_all"), QJsonArray()},
        {QStringLiteral("add_filename"), false},
        {QStringLiteral("filename_namespace"), QStringLiteral("filename")},
        {QStringLiteral("directories"), directories},
        {QStringLiteral("quick_namespaces"), QJsonArray()},
        {QStringLiteral("regexes"), QJsonArray()},
    };
}

// The editor behind "add service"/"edit" on an import folder's filename
// tagging list. The shape matches importing::options::FilenameTaggingOptions.
static bool edit_filename_tagging_options(QWidget *parent, const QString &service_name, const QJsonObject &initial, QJsonObject *result) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("filename tagging for %1").arg(service_name));
    dialog.resize(720, 760);

    auto *layout = new QVBoxLayout(&dialog);

    auto *help = new QLabel(
        QStringLiteral("Tags are read from the path of every successfully imported file. "
                       "Directory slots count from the start of the path (0, 1, 2) or from the end (-3, -2, -1)."),
        &dialog);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *form = new QFormLayout();

    auto *tags_for_all = new QPlainTextEdit(&dialog);
    tags_for_all->setPlaceholderText(QStringLiteral("one tag per line, added to every file"));
    tags_for_all->setMaximumHeight(70);
    tags_for_all->setPlainText([&initial]() {
        QStringList tags;
        for (const QJsonValue &value : initial.value(QStringLiteral("tags_for_all")).toArray()) {
            tags << value.toString();
        }
        return tags.join(QStringLiteral("\n"));
    }());
    form->addRow(QStringLiteral("tags for all files:"), tags_for_all);

    auto *add_filename = new QCheckBox(QStringLiteral("add the filename (without extension)"), &dialog);
    add_filename->setChecked(initial.value(QStringLiteral("add_filename")).toBool(true));
    auto *filename_namespace = new QLineEdit(initial.value(QStringLiteral("filename_namespace")).toString(QStringLiteral("filename")), &dialog);
    filename_namespace->setPlaceholderText(QStringLiteral("namespace (empty means no namespace)"));
    form->addRow(add_filename, filename_namespace);

    layout->addLayout(form);

    auto *directories = new QGroupBox(QStringLiteral("directories"), &dialog);
    auto *directory_grid = new QGridLayout(directories);
    directory_grid->addWidget(new QLabel(QStringLiteral("slot"), directories), 0, 0);
    directory_grid->addWidget(new QLabel(QStringLiteral("use"), directories), 0, 1);
    directory_grid->addWidget(new QLabel(QStringLiteral("namespace"), directories), 0, 2);

    QHash<QString, QCheckBox *> directory_checks;
    QHash<QString, QLineEdit *> directory_namespaces;
    const QJsonObject directory_rules = initial.value(QStringLiteral("directories")).toObject();

    int row = 1;
    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        const QJsonObject rule = directory_rules.value(key).toObject();

        directory_grid->addWidget(new QLabel(key, directories), row, 0);

        auto *check = new QCheckBox(directories);
        check->setChecked(rule.value(QStringLiteral("enabled")).toBool());
        directory_grid->addWidget(check, row, 1);
        directory_checks.insert(key, check);

        auto *tag_namespace = new QLineEdit(rule.value(QStringLiteral("namespace")).toString(), directories);
        tag_namespace->setPlaceholderText(QStringLiteral("empty means no namespace"));
        directory_grid->addWidget(tag_namespace, row, 2);
        directory_namespaces.insert(key, tag_namespace);

        ++row;
    }

    layout->addWidget(directories);

    auto *quick_box = new QGroupBox(QStringLiteral("namespace regexes"), &dialog);
    auto *quick_layout = new QVBoxLayout(quick_box);
    auto *quick = new QTableWidget(quick_box);
    quick->setColumnCount(2);
    quick->setHorizontalHeaderLabels({QStringLiteral("namespace"), QStringLiteral("regex")});
    quick->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    quick->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    quick->verticalHeader()->setVisible(false);
    quick->setMinimumHeight(110);
    quick_layout->addWidget(quick);

    for (const QJsonValue &value : initial.value(QStringLiteral("quick_namespaces")).toArray()) {
        const QJsonObject rule = value.toObject();
        const int index = quick->rowCount();
        quick->insertRow(index);
        quick->setItem(index, 0, new QTableWidgetItem(rule.value(QStringLiteral("namespace")).toString()));
        quick->setItem(index, 1, new QTableWidgetItem(rule.value(QStringLiteral("regex")).toString()));
    }

    auto *quick_buttons = new QHBoxLayout();
    auto *add_quick = new QPushButton(QStringLiteral("add regex"), quick_box);
    auto *remove_quick = new QPushButton(QStringLiteral("remove regex"), quick_box);
    quick_buttons->addWidget(add_quick);
    quick_buttons->addWidget(remove_quick);
    quick_buttons->addStretch(1);
    quick_layout->addLayout(quick_buttons);
    layout->addWidget(quick_box);

    auto *regex_box = new QGroupBox(QStringLiteral("plain regexes"), &dialog);
    auto *regex_layout = new QVBoxLayout(regex_box);
    auto *regexes = new QPlainTextEdit(regex_box);
    regexes->setPlaceholderText(QStringLiteral("one regex per line; every match or capture group becomes a tag"));
    regexes->setMaximumHeight(80);
    regexes->setPlainText([&initial]() {
        QStringList patterns;
        for (const QJsonValue &value : initial.value(QStringLiteral("regexes")).toArray()) {
            patterns << value.toString();
        }
        return patterns.join(QStringLiteral("\n"));
    }());
    regex_layout->addWidget(regexes);
    layout->addWidget(regex_box);

    auto *example = new QLineEdit(initial.value(QStringLiteral("example_path")).toString(), &dialog);
    example->setPlaceholderText(QStringLiteral("example file path for the preview"));
    auto *preview = new QLabel(&dialog);
    preview->setWordWrap(true);
    preview->setObjectName(QStringLiteral("filename_tagging_preview"));

    auto *preview_box = new QGroupBox(QStringLiteral("preview"), &dialog);
    auto *preview_layout = new QVBoxLayout(preview_box);
    preview_layout->addWidget(example);
    preview_layout->addWidget(preview);
    layout->addWidget(preview_box);

    QJsonObject built;

    auto build = [&]() {
        QJsonObject options;
        QJsonArray all_tags;

        for (const QString &line : tags_for_all->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (!trimmed.isEmpty()) {
                all_tags.append(trimmed);
            }
        }

        options.insert(QStringLiteral("tags_for_all"), all_tags);
        options.insert(QStringLiteral("add_filename"), add_filename->isChecked());
        options.insert(QStringLiteral("filename_namespace"), filename_namespace->text().trimmed());

        QJsonObject directory_rules_out;
        for (auto iterator = directory_checks.constBegin(); iterator != directory_checks.constEnd(); ++iterator) {
            directory_rules_out.insert(iterator.key(), QJsonObject{
                {QStringLiteral("enabled"), iterator.value()->isChecked()},
                {QStringLiteral("namespace"), directory_namespaces.value(iterator.key())->text().trimmed()},
            });
        }
        options.insert(QStringLiteral("directories"), directory_rules_out);

        QJsonArray quick_rules;
        for (int index = 0; index < quick->rowCount(); ++index) {
            const QTableWidgetItem *tag_namespace = quick->item(index, 0);
            const QTableWidgetItem *pattern = quick->item(index, 1);

            if (pattern == nullptr || pattern->text().trimmed().isEmpty()) {
                continue;
            }

            quick_rules.append(QJsonObject{
                {QStringLiteral("namespace"), tag_namespace == nullptr ? QString() : tag_namespace->text().trimmed()},
                {QStringLiteral("regex"), pattern->text().trimmed()},
            });
        }
        options.insert(QStringLiteral("quick_namespaces"), quick_rules);

        QJsonArray patterns;
        for (const QString &line : regexes->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (!trimmed.isEmpty()) {
                patterns.append(trimmed);
            }
        }
        options.insert(QStringLiteral("regexes"), patterns);

        built = options;

        if (example->text().trimmed().isEmpty()) {
            preview->setText(QStringLiteral("set an example path to preview the tags"));
        } else {
            const QString tags = filename_tagging_preview(options, example->text());
            preview->setText(tags.isEmpty() ? QStringLiteral("no tags") : tags);
        }
    };

    QObject::connect(tags_for_all, &QPlainTextEdit::textChanged, &dialog, build);
    QObject::connect(add_filename, &QCheckBox::toggled, &dialog, build);
    QObject::connect(filename_namespace, &QLineEdit::textChanged, &dialog, build);
    QObject::connect(example, &QLineEdit::textChanged, &dialog, build);
    QObject::connect(regexes, &QPlainTextEdit::textChanged, &dialog, build);

    for (auto iterator = directory_checks.constBegin(); iterator != directory_checks.constEnd(); ++iterator) {
        QObject::connect(iterator.value(), &QCheckBox::toggled, &dialog, build);
        QObject::connect(directory_namespaces.value(iterator.key()), &QLineEdit::textChanged, &dialog, build);
    }

    QObject::connect(quick, &QTableWidget::cellChanged, &dialog, [build](int, int) { build(); });

    QObject::connect(add_quick, &QPushButton::clicked, &dialog, [quick]() {
        const int index = quick->rowCount();
        quick->insertRow(index);
        quick->setItem(index, 0, new QTableWidgetItem(QString()));
        quick->setItem(index, 1, new QTableWidgetItem(QString()));
        quick->editItem(quick->item(index, 0));
    });
    QObject::connect(remove_quick, &QPushButton::clicked, &dialog, [quick]() {
        const int index = quick->currentRow();
        if (index >= 0) {
            quick->removeRow(index);
        }
    });

    build();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, build]() { build(); dialog.accept(); });

    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }

    build();
    *result = built;

    return true;
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

    // metadata import
    QVBoxLayout *meta_box_layout = nullptr;
    auto *meta_box =
        make_static_box(QStringLiteral("metadata import"), content, &meta_box_layout);

    meta_box_layout->addWidget(new QLabel(QStringLiteral("filename tagging:"), meta_box));

    auto *tagging_list = new QListWidget(meta_box);
    tagging_list->setObjectName(QStringLiteral("import_folder_filename_tagging"));
    meta_box_layout->addWidget(tagging_list, 1);

    auto *tagging_buttons = new QHBoxLayout();
    auto *add_tagging = new QPushButton(QStringLiteral("add"), meta_box);
    auto *edit_tagging = new QPushButton(QStringLiteral("edit"), meta_box);
    auto *remove_tagging = new QPushButton(QStringLiteral("remove"), meta_box);
    tagging_buttons->addWidget(add_tagging);
    tagging_buttons->addWidget(edit_tagging);
    tagging_buttons->addWidget(remove_tagging);
    tagging_buttons->addStretch(1);
    meta_box_layout->addLayout(tagging_buttons);

    meta_box_layout->addWidget(new QLabel(QStringLiteral("sidecar importing:"), meta_box));

    vbox->addWidget(meta_box);

    const QJsonArray tag_services = state.value(QStringLiteral("tag_services")).toArray();
    QHash<QString, QString> service_name_to_key;

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();
        service_name_to_key.insert(service.value(QStringLiteral("name")).toString(), service.value(QStringLiteral("key")).toString());
    }

    QJsonObject filename_tagging = folder.value(QStringLiteral("filename_tagging")).toObject();
    const QString folder_path = folder.value(QStringLiteral("path")).toString();

    auto refresh_tagging = [tagging_list, &filename_tagging, &service_name_to_key]() {
        tagging_list->clear();

        for (auto iterator = filename_tagging.constBegin(); iterator != filename_tagging.constEnd(); ++iterator) {
            const QString name = service_name_to_key.key(iterator.key(), iterator.key());
            auto *item = new QListWidgetItem(
                QStringLiteral("%1 - %2").arg(name, filename_tagging_summary(iterator.value().toObject())),
                tagging_list);
            item->setData(Qt::UserRole, iterator.key());
        }
    };

    auto open_tagging_editor = [this, &dialog, &filename_tagging, &service_name_to_key, folder_path](const QString &key, const QString &name, const QJsonObject &current) {
        QJsonObject initial = current.isEmpty() ? default_filename_tagging_options() : current;
        initial.insert(QStringLiteral("example_path"), folder_path);

        QJsonObject edited;

        if (!edit_filename_tagging_options(&dialog, name, initial, &edited)) {
            return;
        }

        if (filename_tagging_summary(edited) == QStringLiteral("no tags")) {
            filename_tagging.remove(key);
        } else {
            filename_tagging.insert(key, edited);
        }
    };

    connect(add_tagging, &QPushButton::clicked, &dialog, [this, &dialog, &filename_tagging, &service_name_to_key, open_tagging_editor, refresh_tagging]() {
        QStringList available;

        for (auto iterator = service_name_to_key.constBegin(); iterator != service_name_to_key.constEnd(); ++iterator) {
            if (!filename_tagging.contains(iterator.value())) {
                available << iterator.key();
            }
        }

        if (available.isEmpty()) {
            QMessageBox::information(
                this,
                QStringLiteral("filename tagging"),
                QStringLiteral("Create a local tag service first, or every local tag service already has rules."));
            return;
        }

        available.sort();

        bool ok = false;
        const QString name = QInputDialog::getItem(
            &dialog,
            QStringLiteral("add filename tagging"),
            QStringLiteral("tag service:"),
            available,
            0,
            false,
            &ok);

        if (!ok || name.isEmpty()) {
            return;
        }

        open_tagging_editor(service_name_to_key.value(name), name, QJsonObject());
        refresh_tagging();
    });

    connect(edit_tagging, &QPushButton::clicked, &dialog, [&open_tagging_editor, tagging_list, &filename_tagging, &service_name_to_key, refresh_tagging]() {
        const QListWidgetItem *item = tagging_list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();
        open_tagging_editor(key, service_name_to_key.key(key, key), filename_tagging.value(key).toObject());
        refresh_tagging();
    });

    connect(remove_tagging, &QPushButton::clicked, &dialog, [tagging_list, &filename_tagging, refresh_tagging]() {
        const QListWidgetItem *item = tagging_list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();

        if (QMessageBox::question(
                nullptr,
                QStringLiteral("filename tagging"),
                QStringLiteral("remove filename tagging for this service?"))
            != QMessageBox::Yes) {
            return;
        }

        filename_tagging.remove(key);
        refresh_tagging();
    });

    refresh_tagging();

    if (tag_services.isEmpty()) {
        add_tagging->setEnabled(false);
        add_tagging->setToolTip(QStringLiteral("no local tag service exists yet"));
    }

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

    // sidecar exporting
    QVBoxLayout *sidecar_box_layout = nullptr;
    auto *sidecar_box =
        make_static_box(QStringLiteral("sidecar exporting"), content, &sidecar_box_layout);
    auto *sidecar_help = new QLabel(
        QStringLiteral(
            "By default, an export folder will not update pre-existing sidecar files. If you "
            "change the sidecar actions here, or if the metadata has changed and you want those "
            "updates, hit the \"overwrite all sidecars on next run\" checkbox.\n\nYou can force "
            "the export folder to regenerate all your sidecars on every run, but this is an "
            "expensive operation. It is only appropriate for an export folder that runs manually "
            "or rarely.\n\nDO NOT SET YOUR EXPORT FOLDER TO REGEN ALL YOUR SIDECARS EVERY THIRTY "
            "MINUTES."),
        sidecar_box);
    sidecar_help->setWordWrap(true);
    sidecar_box_layout->addWidget(sidecar_help);

    auto *overwrite_next = new QCheckBox(sidecar_box);
    overwrite_next->setChecked(folder.value(QStringLiteral("overwrite_sidecars")).toBool(false));
    auto *overwrite_always = new QCheckBox(sidecar_box);
    overwrite_always->setChecked(
        folder.value(QStringLiteral("always_overwrite_sidecars")).toBool(false));

    auto *sidecar_grid = make_grid();
    auto *overwrite_next_label =
        new QLabel(QStringLiteral("overwrite all sidecars on next run: "), sidecar_box);
    auto *overwrite_always_label = new QLabel(
        QStringLiteral("DANGEROUS: always overwrite all sidecars on every run: "), sidecar_box);
    sidecar_grid->addWidget(overwrite_next_label, 0, 0);
    sidecar_grid->addWidget(overwrite_next, 0, 1, Qt::AlignLeft | Qt::AlignVCenter);
    sidecar_grid->addWidget(overwrite_always_label, 1, 0);
    sidecar_grid->addWidget(overwrite_always, 1, 1, Qt::AlignLeft | Qt::AlignVCenter);
    sidecar_grid->setColumnStretch(1, 1);
    sidecar_box_layout->addLayout(sidecar_grid);

    QString sidecar_format = folder.value(QStringLiteral("sidecar_format")).toString();

    if (sidecar_format != QStringLiteral("json") && sidecar_format != QStringLiteral("text")) {
        sidecar_format = QStringLiteral("none");
    }

    auto sidecar_label_for = [](const QString &format) {
        if (format == QStringLiteral("json")) {
            return QStringLiteral("1 sidecar: tags.json");
        }

        if (format == QStringLiteral("text")) {
            return QStringLiteral("1 sidecar: tags.txt");
        }

        return QStringLiteral("no sidecars");
    };

    auto *sidecar_button = new QPushButton(sidecar_label_for(sidecar_format), sidecar_box);
    sidecar_box_layout->addWidget(sidecar_button);

    connect(sidecar_button, &QPushButton::clicked, &dialog,
            [sidecar_button, &sidecar_format, sidecar_label_for]() {
                QMenu menu(sidecar_button);
                const struct {
                    const char *label;
                    const char *value;
                } entries[] = {{"no sidecars", "none"},
                               {"tags sidecar (json)", "json"},
                               {"tags sidecar (text)", "text"}};

                for (const auto &entry : entries) {
                    const QString value = QString::fromLatin1(entry.value);
                    auto *action = menu.addAction(QString::fromLatin1(entry.label));
                    action->setCheckable(true);
                    action->setChecked(value == sidecar_format);
                    connect(action, &QAction::triggered, sidecar_button, [&sidecar_format,
                                                                         sidecar_button,
                                                                         sidecar_label_for,
                                                                         value]() {
                        sidecar_format = value;
                        sidecar_button->setText(sidecar_label_for(sidecar_format));
                    });
                }

                menu.exec(sidecar_button->mapToGlobal(QPoint(0, sidecar_button->height())));
            });

    auto *test_examples = new QPushButton(QStringLiteral("update test example files"), sidecar_box);
    sidecar_box_layout->addWidget(test_examples, 0, Qt::AlignRight);

    connect(test_examples, &QPushButton::clicked, &dialog, [this, query]() {
        QJsonObject payload;
        payload.insert(QStringLiteral("query"), query->text().trimmed());
        notify(QStringLiteral("export_folder_test_examples"), QString(), json_payload(payload));
    });
    vbox->addWidget(sidecar_box);
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

void HydrusMainWindow::start_tick() {
    if (tick_timer != nullptr) {
        return;
    }

    tick_timer = new QTimer(this);
    tick_timer->setInterval(120);
    connect(tick_timer, &QTimer::timeout, this, [this]() { app_tick(*this); });
    tick_timer->start();
}

void HydrusMainWindow::show_message(const QString &title, const QString &text) {
    QMessageBox::information(this, title, text);
}

void HydrusMainWindow::notify_message(const QString &message) {
    if (status_media != nullptr) {
        status_media->setText(message);
    }

    if (tray != nullptr && tray->isVisible()) {
        tray->showMessage(
            QStringLiteral("Hydrus Client"),
            message,
            QSystemTrayIcon::Information,
            3000);
    }
}

void HydrusMainWindow::schedule_screenshot(const QString &path) {
    int delay = qEnvironmentVariableIntValue("HYDRUS_QT_SCREENSHOT_DELAY_MS");

    if (delay <= 0) {
        delay = 1400;
    }

    QTimer::singleShot(delay, this, [this, path]() {
        QWidget *target = QApplication::activeWindow();

        if (target == nullptr) {
            target = this;
        }

        for (HydrusViewerWindow *viewer : viewers) {
            if (viewer != nullptr && viewer->isVisible()) {
                target = viewer;
                break;
            }
        }

        const bool ok = target->grab().save(path);
        QCoreApplication::quit();

        if (!ok) {
            qWarning("could not save screenshot to %s", path.toUtf8().constData());
        }
    });
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

