#include "duplicates_page.hpp"

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

    build_media_panel(page_splitter, sidebar_scroll, sidebar, sidebar_layout, outer, state);
}