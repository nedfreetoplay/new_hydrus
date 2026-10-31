#pragma once

#include "rust/cxx.h"

#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QFrame>
#include <QGraphicsPixmapItem>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QHash>
#include <QJsonArray>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QResizeEvent>
#include <QScrollArea>
#include <QSpinBox>
#include <QSplashScreen>
#include <QString>
#include <QStringList>
#include <QTabWidget>
#include <QTableWidget>
#include <QTimer>
#include <QToolButton>
#include <QWheelEvent>
#include <QWidget>
#include <memory>
#include "screens.hpp"

// WS-15.1.4: thin cxx aggregator. All declarations live in the topic
// headers below; a single `include!("bridge.hpp")` in bridge.rs covers them.

#include "subscriptions.hpp"
#include "import_folders_manager.hpp"
#include "export_folders_manager.hpp"
#include "main_window.hpp"
#include "bridge_api.hpp"
