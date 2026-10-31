#include "query_page_sidebar.hpp"

// WS-15.1.2b: query-page construction moved verbatim from bridge.cpp (move-only split).

#include "bridge.hpp"
#include "support/support.hpp"
#include "canvas.hpp"
#include "thumbnail_grid.hpp"

#include <QAction>
#include <QComboBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QPalette>
#include <QPoint>
#include <QProgressBar>
#include <QPushButton>
#include <QScrollArea>
#include <QSizePolicy>
#include <QSplitter>
#include <QStyle>
#include <QTabWidget>
#include <QToolButton>
#include <QVBoxLayout>

HydrusQueryPage::HydrusQueryPage(HydrusMainWindow *main_window, const QString &page_id, const QJsonObject &state)
    : QWidget(), main(main_window), id(page_id) {
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

    const HydrusSortOption *initial_sort = find_sort_option(QStringLiteral("size"));
    sort_value = QStringLiteral("size");
    sort_ascending_value = true;

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

    const auto send_sort = [this]() {
        if (main->building) {
            return;
        }

        QJsonObject request;
        request.insert(QStringLiteral("sort"), sort_value);
        request.insert(QStringLiteral("ascending"), sort_ascending_value);
        main->notify(
            QStringLiteral("set_sort"),
            id,
            QString::fromUtf8(QJsonDocument(request).toJson(QJsonDocument::Compact)));
    };

    const auto set_sort_direction = [this, send_sort](bool ascending) {
        const HydrusSortOption *option = find_sort_option(sort_value);
        sort_ascending_value = ascending;
        sort_order->setText(
            QString::fromUtf8(ascending ? option->ascending : option->descending));
        send_sort();
    };

    const auto apply_sort = [this, send_sort](
                                const QString &key, bool ascending, bool notify) {
        const HydrusSortOption *option = find_sort_option(key);
        sort_value = key;
        sort_ascending_value = ascending;
        sort_type->setText(QString::fromUtf8(option->label));
        sort_order->setText(
            QString::fromUtf8(ascending ? option->ascending : option->descending));

        if (notify) {
            send_sort();
        }
    };

    auto *sort_menu = new QMenu(this);

    for (const HydrusSortOption &option : hydrus_sort_options) {
        auto *action = sort_menu->addAction(QString::fromUtf8(option.label));
        action->setData(QString::fromUtf8(option.key));
    }

    connect(sort_menu, &QMenu::triggered, this, [apply_sort](QAction *action) {
        const QString key = action->data().toString();
        apply_sort(key, find_sort_option(key)->default_ascending, true);
    });

    connect(sort_type, &QPushButton::clicked, this, [this, sort_menu]() {
        sort_menu->popup(sort_type->mapToGlobal(QPoint(0, sort_type->height())));
    });

    connect(sort_order, &QPushButton::clicked, this, [this, set_sort_direction]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);

        const HydrusSortOption *option = find_sort_option(sort_value);

        auto *ascending = menu->addAction(QString::fromUtf8(option->ascending));
        auto *descending = menu->addAction(QString::fromUtf8(option->descending));

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

    connect(collect_cog, &QToolButton::clicked, this, [this]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);
        auto *nothing = menu->addAction(QStringLiteral("no collection options available"));
        nothing->setEnabled(false);
        menu->popup(collect_cog->mapToGlobal(QPoint(0, collect_cog->height())));
    });

    auto *collect_row = new QWidget(sidebar_content);
    collect_row->setFixedHeight(24);
    auto *collect_row_layout = new QHBoxLayout(collect_row);
    collect_row_layout->setContentsMargins(0, 0, 0, 0);
    collect_row_layout->setSpacing(2);
    collect_row_layout->addWidget(collect, 1);
    collect_row_layout->addWidget(collect_cog, 0);
    sidebar_layout->addWidget(collect_row);

    QVBoxLayout *search_layout = nullptr;
    QPushButton *search_expander = nullptr;
    search_box = make_static_box(
        QStringLiteral("search"), sidebar_content, &search_layout, &search_expander);
    search_expand = search_expander;
    search_content = search_layout->parentWidget();
    search_box->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    search_box->setFixedHeight(645);
    sidebar_layout->addWidget(search_box);

    connect(search_expand, &QPushButton::clicked, this, [this]() {
        const bool show = !search_content->isVisible();
        search_content->setVisible(show);
        selection_tags_box->setVisible(!show);
        sidebar_filler->setVisible(show);
        search_expand->setIcon(
            style()->standardIcon(show ? QStyle::SP_ArrowUp : QStyle::SP_ArrowDown));
        search_box->setFixedHeight(show ? 645 : 32);
    });

    predicates = new QListWidget(search_box);
    predicates->setSelectionMode(QAbstractItemView::SingleSelection);
    predicates->setUniformItemSizes(true);
    predicates->setFixedHeight(132);
    search_layout->addWidget(predicates);

    auto *input_row = new QWidget(search_box);
    auto *input_layout = new QHBoxLayout(input_row);
    input_layout->setContentsMargins(0, 0, 0, 0);
    input_layout->setSpacing(2);

    search = new QLineEdit(input_row);
    search->setMinimumHeight(24);
    search->setStyleSheet(QStringLiteral("QLineEdit { background-color: #ebf8ff; }"));
    input_layout->addWidget(search, 1);

    auto *paste_button = make_icon_button(
        input_row,
        QStringLiteral("paste"),
        QStringLiteral("You can paste a newline-separated list of regular tags and/or system predicates."));
    auto *favourite_button = make_icon_button(
        input_row, QStringLiteral("star"), QStringLiteral("Load or save a favourite search."));
    auto *clear_button = make_icon_button(
        input_row, QStringLiteral("clear"), QStringLiteral("Clear the search back to an empty page."));
    auto *lock_button = make_icon_button(
        input_row, QStringLiteral("lock"), QStringLiteral("Lock the current files in view to a fixed system:hash."));

    input_layout->addWidget(paste_button, 0);
    input_layout->addWidget(favourite_button, 0);
    input_layout->addWidget(clear_button, 0);
    input_layout->addWidget(lock_button, 0);
    search_layout->addWidget(input_row);

    include_current = new QPushButton(search_box);
    make_on_off_button(
        include_current,
        QStringLiteral("include current tags"),
        QStringLiteral("exclude current tags"),
        true);
    include_current->setMinimumHeight(24);
    include_current->setToolTip(QStringLiteral("select whether to include current tags in the search"));

    include_pending = new QPushButton(search_box);
    make_on_off_button(
        include_pending,
        QStringLiteral("include pending tags"),
        QStringLiteral("exclude pending tags"),
        true);
    include_pending->setMinimumHeight(24);
    include_pending->setToolTip(QStringLiteral("select whether to include pending tags in the search"));

    auto *button_row_1 = new QWidget(search_box);
    auto *button_row_1_layout = new QHBoxLayout(button_row_1);
    button_row_1_layout->setContentsMargins(0, 0, 0, 0);
    button_row_1_layout->setSpacing(2);
    button_row_1_layout->addWidget(include_current, 1);
    button_row_1_layout->addWidget(include_pending, 1);
    search_layout->addWidget(button_row_1);

    search_pause_play = new QPushButton(search_box);
    make_on_off_button(
        search_pause_play,
        QStringLiteral("searching immediately"),
        QStringLiteral("search paused"),
        true);
    search_pause_play->setMinimumHeight(24);
    search_pause_play->setToolTip(
        QStringLiteral("select whether to renew the search as soon as a new predicate is entered"));

    or_button = new QPushButton(QStringLiteral("OR"), search_box);
    or_button->setMinimumHeight(24);
    or_button->setFixedWidth(75);

    auto *button_row_2 = new QWidget(search_box);
    auto *button_row_2_layout = new QHBoxLayout(button_row_2);
    button_row_2_layout->setContentsMargins(0, 0, 0, 0);
    button_row_2_layout->setSpacing(2);
    button_row_2_layout->addWidget(search_pause_play, 1);
    button_row_2_layout->addWidget(or_button, 0);
    search_layout->addWidget(button_row_2);

    location_value = QStringLiteral("my files");
    location = new QPushButton(location_value, search_box);
    location->setMinimumHeight(24);
    location->setToolTip(QStringLiteral("select the file domain for this search"));

    tag_service_value.clear();
    tag_service = new QPushButton(QStringLiteral("all known tags"), search_box);
    tag_service->setMinimumHeight(24);
    tag_service->setToolTip(QStringLiteral("select the tag domain for this search"));

    auto *button_row_3 = new QWidget(search_box);
    auto *button_row_3_layout = new QHBoxLayout(button_row_3);
    button_row_3_layout->setContentsMargins(0, 0, 0, 0);
    button_row_3_layout->setSpacing(2);
    button_row_3_layout->addWidget(location, 1);
    button_row_3_layout->addWidget(tag_service, 1);
    search_layout->addWidget(button_row_3);

    const auto send_location = [this](const QString &value) {
        location_value = value;
        location->setText(value);

        if (!main->building) {
            main->notify(QStringLiteral("set_location"), id, value);
        }
    };

    connect(location, &QPushButton::clicked, this, [this, send_location]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);

        for (const QString &text :
             {QStringLiteral("my files"), QStringLiteral("all local files"), QStringLiteral("trash")}) {
            auto *action = menu->addAction(text);

            connect(action, &QAction::triggered, this, [send_location, text]() {
                send_location(text);
            });
        }

        menu->popup(location->mapToGlobal(QPoint(0, location->height())));
    });

    const auto send_tag_service = [this](const QString &label) {
        tag_service_value = label == QStringLiteral("all known tags") ? QString() : label;
        tag_service->setText(label);

        if (!main->building) {
            main->notify(QStringLiteral("set_tag_service"), id, tag_service_value);
        }
    };

    connect(tag_service, &QPushButton::clicked, this, [this, send_tag_service]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);

        auto *all_known = menu->addAction(QStringLiteral("all known tags"));

        connect(all_known, &QAction::triggered, this, [send_tag_service]() {
            send_tag_service(QStringLiteral("all known tags"));
        });

        for (const QString &name : tag_services) {
            auto *action = menu->addAction(name);

            connect(action, &QAction::triggered, this, [send_tag_service, name]() {
                send_tag_service(name);
            });
        }

        menu->popup(tag_service->mapToGlobal(QPoint(0, tag_service->height())));
    });

    autocomplete_tabs = new QTabWidget(search_box);
    autocomplete_tabs->setStyleSheet(QStringLiteral(
        "QTabWidget::pane { border: 1px solid #b8b8b8; background: #ffffff; }"
        "QTabBar::tab { background: #f0f0f0; }"
        "QTabBar::tab:selected { background: #ffffff; }"
        "QListWidget { background: #ffffff; border: none; }"
        "QListWidget::item:selected { background: #a0522d; color: #ffffff; }"));

    results_list = new QListWidget(autocomplete_tabs);
    results_list->setSelectionMode(QAbstractItemView::SingleSelection);
    results_list->setUniformItemSizes(true);
    fill_system_predicates(results_list);

    favourites_list = new QListWidget(autocomplete_tabs);
    children_list = new QListWidget(autocomplete_tabs);

    autocomplete_tabs->addTab(results_list, QStringLiteral("results"));
    autocomplete_tabs->addTab(favourites_list, QStringLiteral("favourites"));
    autocomplete_tabs->addTab(children_list, QStringLiteral("children"));
    search_layout->addWidget(autocomplete_tabs, 1);

    const auto send_search = [this]() {
        if (main->building) {
            return;
        }

        const QByteArray page_bytes = id.toUtf8();
        const QByteArray query_bytes = search->text().toUtf8();
        app_action(
            *main,
            rust::Str("search", 6),
            rust::Str(page_bytes.constData(), page_bytes.size()),
            rust::Str(query_bytes.constData(), query_bytes.size()));
    };

    connect(search, &QLineEdit::returnPressed, this, send_search);

    connect(results_list, &QListWidget::itemDoubleClicked, this, [this, send_search](QListWidgetItem *item) {
        if (item == nullptr) {
            return;
        }

        search->setText(item->text());
        send_search();
    });

    connect(clear_button, &QToolButton::clicked, this, [this, send_search]() {
        search->clear();
        predicates->clear();
        fill_system_predicates(results_list);
        send_search();
    });

    connect(search, &QLineEdit::textEdited, this, [this](const QString &text) {
        text_before_completion = text;
        request_autocomplete();
    });

    autocomplete = nullptr;

    QVBoxLayout *tags_layout = nullptr;
    auto *tags_box = make_static_box(
        QStringLiteral("selection tags"), sidebar_content, &tags_layout);
    selection_tags_box = tags_box;
    tags_box->setVisible(false);
    sidebar_layout->addWidget(tags_box, 1);

    sidebar_filler = new QWidget(sidebar_content);
    sidebar_filler->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Expanding);
    sidebar_layout->addWidget(sidebar_filler, 1);

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
    tags_layout->addWidget(selection_tags, 1);

    const auto sort_selection = [this]() {
        selection_tags->sortItems(
            tag_sort_order->text() == QStringLiteral("a-z") ? Qt::AscendingOrder
                                                           : Qt::DescendingOrder);
    };

    connect(tag_sort_order, &QPushButton::clicked, this, [this, sort_selection]() {
        tag_sort_order->setText(
            tag_sort_order->text() == QStringLiteral("a-z") ? QStringLiteral("z-a")
                                                            : QStringLiteral("a-z"));
        sort_selection();
    });

    connect(tag_sort_type, &QPushButton::clicked, this, [this]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);

        for (const QString &text : {QStringLiteral("sort by tag"),
                                    QStringLiteral("sort by namespace"),
                                    QStringLiteral("sort by incidence")}) {
            auto *action = menu->addAction(text);

            connect(action, &QAction::triggered, this, [this, text]() {
                tag_sort_type->setText(text);
            });
        }

        menu->popup(tag_sort_type->mapToGlobal(QPoint(0, tag_sort_type->height())));
    });

    connect(tag_sort_namespace, &QPushButton::clicked, this, [this]() {
        auto *menu = new QMenu(this);
        menu->setAttribute(Qt::WA_DeleteOnClose);

        for (const QString &text : {QStringLiteral("namespace (user)"),
                                    QStringLiteral("namespace (creator)"),
                                    QStringLiteral("no namespace")}) {
            auto *action = menu->addAction(text);

            connect(action, &QAction::triggered, this, [this, text]() {
                tag_sort_namespace->setText(text);
            });
        }

        menu->popup(tag_sort_namespace->mapToGlobal(QPoint(0, tag_sort_namespace->height())));
    });

    auto *vertical = new QSplitter(Qt::Vertical, splitter);
    vertical->addWidget(sidebar_scroll);

    preview_frame = new QFrame(vertical);
    preview_frame->setFrameStyle(QFrame::Panel | QFrame::Sunken);
    preview_frame->setLineWidth(2);
    auto *preview_layout = new QVBoxLayout(preview_frame);
    preview_layout->setContentsMargins(0, 0, 0, 0);
    preview = new HydrusCanvas(preview_frame);
    preview->set_background_mode(QStringLiteral("white"));
    preview->setMinimumHeight(120);
    preview_layout->addWidget(preview);
    vertical->addWidget(preview_frame);
    vertical->setStretchFactor(0, 1);
    vertical->setStretchFactor(1, 0);
    vertical->setSizes({900, 280});
    preview_frame->setVisible(false);

    splitter->addWidget(vertical);

    results = new QWidget(splitter);
    results->setAutoFillBackground(true);
    QPalette results_palette = results->palette();
    results_palette.setColor(QPalette::Window, Qt::white);
    results->setPalette(results_palette);
    auto *results_layout = new QVBoxLayout(results);
    results_layout->setContentsMargins(0, 0, 0, 0);
    results_layout->setSpacing(0);

    grid = new HydrusThumbnailGrid(main, this);
    results_layout->addWidget(grid, 1);

    loading_panel = new QWidget(results);
    loading_panel->setObjectName(QStringLiteral("HydrusLoadingPanel"));
    loading_panel->setStyleSheet(
        QStringLiteral("QWidget#HydrusLoadingPanel { background: rgba(235, 235, 235, 200); }"));
    auto *loading_layout = new QVBoxLayout(loading_panel);
    loading_layout->addStretch(1);
    loading_text = new QLabel(QStringLiteral("loading files..."), loading_panel);
    loading_text->setAlignment(Qt::AlignHCenter);
    loading_text->setStyleSheet(
        QStringLiteral("font-size: 15px; color: #202020; background: transparent;"));
    loading_layout->addWidget(loading_text);
    auto *loading_bar = new QProgressBar(loading_panel);
    loading_bar->setRange(0, 0);
    loading_bar->setTextVisible(false);
    loading_bar->setFixedWidth(240);
    loading_bar->setStyleSheet(
        QStringLiteral("QProgressBar { background: #f0f0f0; border: 1px solid #b0b0b0; }"
                       "QProgressBar::chunk { background: #4a90d9; }"));
    loading_layout->addWidget(loading_bar, 0, Qt::AlignHCenter);
    loading_layout->addStretch(2);
    loading_panel->hide();

    splitter->addWidget(results);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({412, 884});

    layout->addWidget(splitter);

    update_state(state);
}
