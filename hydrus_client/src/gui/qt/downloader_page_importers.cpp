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

void HydrusDownloaderPage::build_urls_panels(QVBoxLayout *main_layout) {
    const bool urls = kind == QStringLiteral("urls");
    if (urls) {
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
            make_row_icon(pause_row, QStringLiteral("pause"), QStringLiteral("pause/play files"));
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
        file_cancel = make_row_icon(file_net_gauge_row, QStringLiteral("stop"), QStringLiteral("cancel"));
        file_net_gauge_layout->addWidget(file_cancel, 0);
        file_cog = make_row_icon(
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
            make_row_icon(search_net_gauge_row, QStringLiteral("stop"), QStringLiteral("cancel"));
        search_net_gauge_layout->addWidget(search_cancel, 0);
        search_cog = make_row_icon(
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
        url_paste = make_row_icon(
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
    }
}

void HydrusDownloaderPage::build_simple_panels(QVBoxLayout *main_layout) {
    const bool simple = kind == QStringLiteral("simple");
    if (simple) {
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
                simple_paste = make_row_icon(
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
                    make_row_icon(formula_row, QStringLiteral("gear"), QStringLiteral("formula options"));
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
}

void HydrusDownloaderPage::build_import_options(QVBoxLayout *main_layout, const QJsonObject &state) {
    const bool urls = kind == QStringLiteral("urls");
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
}