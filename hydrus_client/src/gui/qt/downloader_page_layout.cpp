#include "downloader_page.hpp"

// WS-15.1.2c: moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QDateTime>
#include <QFont>
#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMessageBox>
#include <QPalette>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QSignalBlocker>
#include <QSizePolicy>
#include <QSpinBox>
#include <QSplitter>
#include <QStyle>
#include <QTableWidget>
#include <QTabWidget>
#include <QTimer>
#include <QToolButton>
#include <QVBoxLayout>

QToolButton *HydrusDownloaderPage::make_row_icon(QWidget *parent, const QString &icon_kind, const QString &tooltip) {
        auto *button = new QToolButton(parent);
        button->setIcon(hydrus_small_icon(icon_kind));
        button->setIconSize(QSize(16, 16));
        button->setFixedSize(28, 24);
        button->setAutoRaise(true);
        button->setFocusPolicy(Qt::NoFocus);
        button->setToolTip(tooltip);
        return button;
}

QFrame *HydrusDownloaderPage::make_status_box(
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

        *pause_button = make_row_icon(status_row, QStringLiteral("pause"), pause_tip);
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

        *cog_button = make_row_icon(network_row, QStringLiteral("gear"), QStringLiteral("network job options"));
        network_layout->addWidget(*cog_button, 0);
        box_layout->addWidget(network_row);

        outer_layout->addWidget(box);

        return box;
}

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


    build_gallery_panel(main_layout);
    build_watcher_panel(main_layout);
    build_urls_panels(main_layout);
    build_simple_panels(main_layout);
    build_import_options(main_layout, state);
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

    build_highlight_panel(sidebar_layout, state);
    build_footer(sidebar_layout);

    fill_sites(state.value(QStringLiteral("sites")).toArray());

    // Make the scroll area aware of the real content minimum; otherwise Qt
    // squeezes the selection tags box below the viewport instead of scrolling.
    sidebar_content->setMinimumHeight(sidebar_layout->minimumSize().height());

    update_state(state);
}
const QString &HydrusDownloaderPage::page_id() const {
    return id;
}

void HydrusDownloaderPage::build_footer(QVBoxLayout *sidebar_layout) {
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
}